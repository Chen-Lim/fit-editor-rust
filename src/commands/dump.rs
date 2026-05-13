use std::path::Path;

use colored::Colorize;
use fit::{Decoder, Message, Value};

use crate::error::{read_bounded, CliError, DEFAULT_MAX_FILE_SIZE};

pub fn run(
    file: &str,
    message_filter: Option<&str>,
    field_filter: Option<&str>,
    limit: Option<usize>,
    raw: bool,
    compact: bool,
    verbose: bool,
) -> Result<(), CliError> {
    let path = Path::new(file);
    let bytes = read_bounded(path, DEFAULT_MAX_FILE_SIZE)?;

    if !fit::is_fit(&bytes) {
        return Err(CliError::NotFit(file.to_string()));
    }

    let mut builder = Decoder::builder(&bytes);
    if raw {
        builder = builder
            .apply_scale_and_offset(false)
            .convert_datetime(false)
            .convert_types_to_strings(false)
            .expand_components(false)
            .expand_subfields(false);
    }

    let (messages, errors) = builder.build().read_all();

    if !errors.is_empty() && verbose {
        for err in &errors {
            eprintln!("{} {err:?}", "⚠".yellow());
        }
    }

    let fields: Option<Vec<&str>> = field_filter.map(|f| f.split(',').collect());
    let mut shown = 0usize;

    for (idx, msg) in messages.iter().enumerate() {
        if let Some(filter) = message_filter {
            if msg.name != filter {
                continue;
            }
        }

        if compact {
            print_compact(idx, msg, &fields);
        } else {
            print_full(idx, msg, &fields);
        }

        shown += 1;
        if let Some(max) = limit {
            if shown >= max {
                break;
            }
        }
    }

    Ok(())
}

fn print_full(idx: usize, msg: &Message, field_filter: &Option<Vec<&str>>) {
    println!(
        "[{}] {}",
        idx.to_string().dimmed(),
        msg.name.cyan().bold()
    );

    for field in &msg.fields {
        if let Some(filter) = field_filter {
            if !filter.contains(&field.name.as_str()) {
                continue;
            }
        }

        let value_str = format_value(&field.value);
        let unit_str = field
            .units
            .as_ref()
            .map(|u| format!(" {u}"))
            .unwrap_or_default();

        println!("  {} = {}{}", field.name.green(), value_str, unit_str.dimmed());
    }
    println!();
}

fn print_compact(idx: usize, msg: &Message, field_filter: &Option<Vec<&str>>) {
    let parts: Vec<String> = msg
        .fields
        .iter()
        .filter(|f| {
            field_filter
                .as_ref()
                .map_or(true, |ff| ff.contains(&f.name.as_str()))
        })
        .map(|f| format!("{}={}", f.name, format_value(&f.value)))
        .collect();

    println!("[{}] {}: {}", idx, msg.name.cyan(), parts.join("  "));
}

fn format_value(value: &Value) -> String {
    match value {
        Value::Invalid => "<invalid>".dimmed().to_string(),
        Value::SInt(v) => v.to_string(),
        Value::UInt(v) => v.to_string(),
        Value::Float(v) => format!("{v:.4}"),
        Value::String(s) => format!("\"{s}\""),
        Value::Bytes(b) => format!("<{} bytes>", b.len()),
        Value::Bool(b) => b.to_string(),
        Value::Enum(name) => name.to_string(),
        Value::DateTime(dt) => dt.to_rfc3339(),
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(format_value).collect();
            format!("[{}]", items.join(", "))
        }
    }
}
