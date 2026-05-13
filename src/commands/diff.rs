use std::path::Path;

use colored::Colorize;
use fit::{Decoder, Message};

use crate::error::CliError;

pub fn run(
    file1: &str,
    file2: &str,
    ignore_timestamps: bool,
    message_filter: Option<&str>,
) -> Result<(), CliError> {
    let bytes1 = std::fs::read(Path::new(file1))?;
    let bytes2 = std::fs::read(Path::new(file2))?;

    if !fit::is_fit(&bytes1) {
        return Err(CliError::NotFit(file1.to_string()));
    }
    if !fit::is_fit(&bytes2) {
        return Err(CliError::NotFit(file2.to_string()));
    }

    let (msgs1, _) = Decoder::builder(&bytes1).build().read_all();
    let (msgs2, _) = Decoder::builder(&bytes2).build().read_all();

    let filter = |m: &&Message| message_filter.map_or(true, |f| m.name == f);
    let filtered1: Vec<&Message> = msgs1.iter().filter(filter).collect();
    let filtered2: Vec<&Message> = msgs2.iter().filter(filter).collect();

    let max_len = filtered1.len().max(filtered2.len());
    let mut diffs_found = 0;

    println!("{} vs {}", file1.red(), file2.green());
    println!();

    for i in 0..max_len {
        match (filtered1.get(i), filtered2.get(i)) {
            (Some(a), Some(b)) => {
                if a.name != b.name {
                    println!("[{}] --- {} {}", i, a.name.red(), "(type changed)".dimmed());
                    println!("[{}] +++ {} {}", i, b.name.green(), "(type changed)".dimmed());
                    println!();
                    diffs_found += 1;
                    continue;
                }

                let field_diffs = compare_messages(a, b, ignore_timestamps);
                if !field_diffs.is_empty() {
                    println!("[{}] @@ {} @@", i, a.name.cyan().bold());
                    for (field_name, old_val, new_val) in &field_diffs {
                        println!("  - {} = {}", field_name.red(), old_val);
                        println!("  + {} = {}", field_name.green(), new_val);
                    }
                    println!();
                    diffs_found += 1;
                }
            }
            (Some(a), None) => {
                println!("[{}] --- {} (only in {})", i, a.name.red(), file1);
                diffs_found += 1;
            }
            (None, Some(b)) => {
                println!("[{}] +++ {} (only in {})", i, b.name.green(), file2);
                diffs_found += 1;
            }
            (None, None) => unreachable!(),
        }
    }

    if diffs_found == 0 {
        println!("{}", "No differences found".green());
    } else {
        println!(
            "{} difference(s) found",
            diffs_found.to_string().yellow()
        );
    }

    Ok(())
}

fn compare_messages(
    a: &Message,
    b: &Message,
    ignore_timestamps: bool,
) -> Vec<(String, String, String)> {
    let mut diffs = Vec::new();

    let all_fields: std::collections::BTreeSet<&str> = a
        .fields
        .iter()
        .map(|f| f.name.as_str())
        .chain(b.fields.iter().map(|f| f.name.as_str()))
        .collect();

    for field_name in all_fields {
        if ignore_timestamps && field_name == "timestamp" {
            continue;
        }

        let val_a = a
            .field(field_name)
            .map(|f| format_value(&f.value))
            .unwrap_or_else(|| "<missing>".to_string());

        let val_b = b
            .field(field_name)
            .map(|f| format_value(&f.value))
            .unwrap_or_else(|| "<missing>".to_string());

        if val_a != val_b {
            diffs.push((field_name.to_string(), val_a, val_b));
        }
    }

    diffs
}

fn format_value(value: &fit::Value) -> String {
    match value {
        fit::Value::Invalid => "<invalid>".to_string(),
        fit::Value::SInt(v) => v.to_string(),
        fit::Value::UInt(v) => v.to_string(),
        fit::Value::Float(v) => format!("{v:.4}"),
        fit::Value::String(s) => format!("\"{s}\""),
        fit::Value::Bytes(b) => format!("<{} bytes>", b.len()),
        fit::Value::Bool(b) => b.to_string(),
        fit::Value::Enum(name) => name.to_string(),
        fit::Value::DateTime(dt) => dt.to_rfc3339(),
        fit::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(format_value).collect();
            format!("[{}]", items.join(", "))
        }
    }
}
