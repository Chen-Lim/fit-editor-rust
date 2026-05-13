use std::path::Path;

use colored::Colorize;
use fit::profile::{mesg_info_by_num, MesgNum};
use fit::{Encoder, Field, FieldKind, Message, Value};

use crate::error::CliError;

pub fn run(file: &str, output: &str) -> Result<(), CliError> {
    let path = Path::new(file);
    let json_str = std::fs::read_to_string(path)?;
    let json: serde_json::Value = serde_json::from_str(&json_str)?;

    let messages = parse_messages_from_json(&json)?;

    let encoded = Encoder::new().encode(&messages)?;
    fit::check_integrity(&encoded)?;

    std::fs::write(output, &encoded)?;

    println!(
        "{} → {} ({} bytes, {} messages)",
        file.dimmed(),
        output.green().bold(),
        encoded.len(),
        messages.len()
    );

    Ok(())
}

fn parse_messages_from_json(json: &serde_json::Value) -> Result<Vec<Message>, CliError> {
    let arr = json
        .get("messages")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            CliError::InvalidFieldPath("JSON must have a 'messages' array".into())
        })?;

    let mut messages = Vec::new();
    for (idx, entry) in arr.iter().enumerate() {
        let msg = json_value_to_message(entry).map_err(|e| {
            CliError::InvalidFieldPath(format!("message[{idx}]: {e}"))
        })?;
        messages.push(msg);
    }
    Ok(messages)
}

fn json_value_to_message(val: &serde_json::Value) -> Result<Message, String> {
    let msg_type = val
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or("missing 'type' field")?;

    let fields_val = val
        .get("fields")
        .and_then(|v| v.as_object())
        .ok_or("missing 'fields' object")?;

    let mesg_num = MesgNum::from_name(msg_type)
        .ok_or_else(|| format!("unknown message type: {msg_type}"))?;
    let global_mesg_num = mesg_num as u16;

    let info = mesg_info_by_num(global_mesg_num)
        .ok_or_else(|| format!("no profile info for message type: {msg_type}"))?;

    let mut fields = Vec::new();
    for (key, fval) in fields_val {
        let field_info = info
            .field_by_name(key)
            .ok_or_else(|| format!("unknown field '{key}' in message '{msg_type}'"))?;

        let value = json_to_fit_value(fval, field_info);

        fields.push(Field {
            name: key.clone(),
            kind: FieldKind::Standard {
                field_def_num: field_info.field_def_num,
            },
            value,
            units: field_info.units.map(String::from),
        });
    }

    Ok(Message {
        global_mesg_num,
        name: info.name,
        fields,
    })
}

fn json_to_fit_value(val: &serde_json::Value, fi: &fit::profile::FieldInfo) -> Value {
    match val {
        serde_json::Value::Null => Value::Invalid,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            let f = n.as_f64().unwrap_or(0.0);
            // Scaled fields store physical values as Float so the encoder
            // can reverse: raw = (physical + offset) * scale
            let has_scale = fi.scale.is_some_and(|s| s != 1.0);
            let has_offset = fi.offset.is_some_and(|o| o != 0.0);
            if has_scale || has_offset {
                return Value::Float(f);
            }
            match fi.type_name {
                "uint8" | "uint8z" | "uint16" | "uint16z" | "uint32" | "uint32z"
                | "uint64" | "uint64z" => Value::UInt(f as u64),
                "sint8" | "sint16" | "sint32" | "sint64" => Value::SInt(f as i64),
                "float32" | "float64" => Value::Float(f),
                _ => Value::Float(f),
            }
        }
        serde_json::Value::String(s) => {
            if is_datetime_type(fi.type_name) {
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                    Value::DateTime(dt.with_timezone(&chrono::Utc))
                } else {
                    Value::String(s.clone())
                }
            } else if !is_base_type(fi.type_name) {
                // Treat as enum name
                Value::Enum(Box::leak(s.clone().into_boxed_str()))
            } else {
                Value::String(s.clone())
            }
        }
        serde_json::Value::Array(_) => Value::Invalid,
        serde_json::Value::Object(_) => Value::Invalid,
    }
}

fn is_datetime_type(name: &str) -> bool {
    matches!(name, "date_time" | "local_date_time" | "localtime_into_day")
}

fn is_base_type(name: &str) -> bool {
    matches!(
        name,
        "enum"
            | "sint8"
            | "uint8"
            | "sint16"
            | "uint16"
            | "sint32"
            | "uint32"
            | "string"
            | "float32"
            | "float64"
            | "uint8z"
            | "uint16z"
            | "uint32z"
            | "byte"
            | "sint64"
            | "uint64"
            | "uint64z"
    )
}
