use std::path::Path;

use colored::Colorize;
use fit::{Decoder, Encoder, Value};

use crate::error::CliError;

pub fn run(
    file: &str,
    output_prefix: &str,
    at_timestamp: Option<&str>,
    at_index: Option<usize>,
) -> Result<(), CliError> {
    let path = Path::new(file);
    let bytes = std::fs::read(path)?;

    if !fit::is_fit(&bytes) {
        return Err(CliError::NotFit(file.to_string()));
    }

    let (messages, _errors) = Decoder::builder(&bytes).build().read_all();

    let split_point = if let Some(ts_str) = at_timestamp {
        let target = chrono::DateTime::parse_from_rfc3339(ts_str)
            .map_err(|_| {
                CliError::InvalidFieldPath(format!("invalid timestamp: {ts_str}"))
            })?
            .with_timezone(&chrono::Utc);

        messages
            .iter()
            .position(|m| {
                m.field("timestamp")
                    .map(|f| matches!(&f.value, Value::DateTime(dt) if *dt >= target))
                    .unwrap_or(false)
            })
            .unwrap_or(messages.len())
    } else if let Some(idx) = at_index {
        idx.min(messages.len())
    } else {
        return Err(CliError::InvalidFieldPath(
            "must specify --at <timestamp> or --at-index <n>".into(),
        ));
    };

    let (part1, part2) = messages.split_at(split_point);

    let out1 = format!("{output_prefix}_part1.fit");
    let out2 = format!("{output_prefix}_part2.fit");

    if !part1.is_empty() {
        let enc1 = Encoder::new().encode(part1)?;
        std::fs::write(&out1, &enc1)?;
    }
    if !part2.is_empty() {
        let enc2 = Encoder::new().encode(part2)?;
        std::fs::write(&out2, &enc2)?;
    }

    println!(
        "{} messages in {}, {} messages in {}",
        part1.len().to_string().cyan(),
        out1.green(),
        part2.len().to_string().cyan(),
        out2.green(),
    );

    Ok(())
}
