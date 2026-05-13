use std::path::Path;

use colored::Colorize;
use fit::{Decoder, Encoder, Message, Value};

use crate::error::{read_bounded, CliError, DEFAULT_MAX_FILE_SIZE};

pub fn run(
    file: &str,
    output: &str,
    set_fields: &[String],
    remove_messages: &[String],
) -> Result<(), CliError> {
    let path = Path::new(file);
    let bytes = read_bounded(path, DEFAULT_MAX_FILE_SIZE)?;

    if !fit::is_fit(&bytes) {
        return Err(CliError::NotFit(file.to_string()));
    }

    let (mut messages, _errors) = Decoder::builder(&bytes).build().read_all();

    let original_count = messages.len();

    // Remove message types
    if !remove_messages.is_empty() {
        messages.retain(|m| !remove_messages.contains(&m.name.to_string()));
    }

    // Apply --set field=path=value
    let parsed: Vec<FieldEdit> = set_fields
        .iter()
        .map(|s| parse_set_arg(s))
        .collect::<Result<Vec<_>, _>>()?;

    for edit in &parsed {
        apply_edit(&mut messages, edit)?;
    }

    let encoded = Encoder::new().encode(&messages)?;
    fit::check_integrity(&encoded)?;

    std::fs::write(output, &encoded)?;

    let removed = original_count - messages.len();
    if removed > 0 {
        println!(
            "Removed {} message(s), {} remaining",
            removed.to_string().red(),
            messages.len().to_string().green()
        );
    }
    if !parsed.is_empty() {
        println!(
            "Applied {} field edit(s)",
            parsed.len().to_string().green()
        );
    }
    println!(
        "{} → {} ({} bytes)",
        file.dimmed(),
        output.green().bold(),
        encoded.len()
    );

    Ok(())
}

struct FieldEdit {
    /// Message type name (e.g. "session", "file_id")
    msg_type: String,
    /// Optional message index (e.g. "session[2]" → Some(2))
    msg_index: Option<usize>,
    /// Field name (e.g. "total_distance")
    field_name: String,
    /// New value as string
    value: String,
}

fn parse_set_arg(arg: &str) -> Result<FieldEdit, CliError> {
    let eq_pos = arg.rfind('=').ok_or_else(|| {
        CliError::InvalidFieldPath(format!("missing '=' in --set argument: {arg}"))
    })?;

    let path = &arg[..eq_pos];
    let value = arg[eq_pos + 1..].to_string();

    // Parse path: "session.total_distance" or "session[2].total_distance"
    let dot_pos = path.rfind('.').ok_or_else(|| {
        CliError::InvalidFieldPath(format!(
            "field path must be <message>.<field>: {path}"
        ))
    })?;

    let msg_part = &path[..dot_pos];
    let field_name = path[dot_pos + 1..].to_string();

    // Check for index like "session[2]"
    let (msg_type, msg_index) = if let Some(bracket) = msg_part.find('[') {
        let name = msg_part[..bracket].to_string();
        let idx_str = msg_part[bracket + 1..].trim_end_matches(']');
        let idx: usize = idx_str.parse().map_err(|_| {
            CliError::InvalidFieldPath(format!("invalid message index in: {msg_part}"))
        })?;
        (name, Some(idx))
    } else {
        (msg_part.to_string(), None)
    };

    Ok(FieldEdit {
        msg_type,
        msg_index,
        field_name,
        value,
    })
}

fn apply_edit(messages: &mut [Message], edit: &FieldEdit) -> Result<(), CliError> {
    let mut matched = 0usize;

    for msg in messages.iter_mut() {
        if msg.name != edit.msg_type {
            continue;
        }

        if let Some(idx) = edit.msg_index {
            if matched != idx {
                matched += 1;
                continue;
            }
        }

        let field = msg
            .fields
            .iter_mut()
            .find(|f| f.name == edit.field_name)
            .ok_or_else(|| {
                CliError::InvalidFieldPath(format!(
                    "field '{}' not found in message '{}'",
                    edit.field_name, edit.msg_type
                ))
            })?;

        field.value = parse_field_value(&edit.value, &field.value);
        return Ok(());
    }

    if let Some(idx) = edit.msg_index {
        Err(CliError::InvalidFieldPath(format!(
            "message '{}[{}]' not found",
            edit.msg_type, idx
        )))
    } else {
        Err(CliError::InvalidFieldPath(format!(
            "message '{}' not found",
            edit.msg_type
        )))
    }
}

fn parse_field_value(s: &str, template: &Value) -> Value {
    match template {
        Value::UInt(_) => s.parse::<u64>().map(Value::UInt).unwrap_or(Value::Invalid),
        Value::SInt(_) => s.parse::<i64>().map(Value::SInt).unwrap_or(Value::Invalid),
        Value::Float(_) => s.parse::<f64>().map(Value::Float).unwrap_or(Value::Invalid),
        Value::Bool(_) => s.parse::<bool>().map(Value::Bool).unwrap_or(Value::Invalid),
        Value::Enum(_) => Value::Enum(s.to_string()),
        Value::DateTime(_) => chrono::DateTime::parse_from_rfc3339(s)
            .map(|dt| Value::DateTime(dt.with_timezone(&chrono::Utc)))
            .unwrap_or(Value::Invalid),
        Value::String(_) => Value::String(s.to_string()),
        _ => Value::String(s.to_string()),
    }
}
