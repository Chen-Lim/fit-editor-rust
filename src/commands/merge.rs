use std::path::Path;

use colored::Colorize;
use fit::{Decoder, Encoder, Message};

use crate::error::{read_bounded, CliError, DEFAULT_MAX_FILE_SIZE};

/// Metadata-only message types that should only appear once (from the first file).
const METADATA_TYPES: &[&str] = &[
    "file_id",
    "file_creator",
    "device_info",
    "developer_data_id",
    "field_description",
];

pub fn run(files: &[String], output: &str) -> Result<(), CliError> {
    if files.len() < 2 {
        return Err(CliError::BadUsage("merge requires at least 2 files".into()));
    }

    let mut all_messages: Vec<Message> = Vec::new();

    for (i, file) in files.iter().enumerate() {
        let path = Path::new(file);
        let bytes = read_bounded(path, DEFAULT_MAX_FILE_SIZE)?;

        if !fit::is_fit(&bytes) {
            return Err(CliError::NotFit(file.clone()));
        }

        let (messages, _errors) = Decoder::builder(&bytes).build().read_all();

        if i > 0 {
            // Strip metadata-only messages from subsequent files
            let filtered: Vec<Message> = messages
                .into_iter()
                .filter(|m| !METADATA_TYPES.contains(&m.name))
                .collect();
            all_messages.extend(filtered);
        } else {
            all_messages.extend(messages);
        }
    }

    // Sort by timestamp where available
    all_messages.sort_by(|a, b| {
        let ts_a = extract_timestamp(a);
        let ts_b = extract_timestamp(b);
        ts_a.cmp(&ts_b)
    });

    let encoded = Encoder::new().encode(&all_messages)?;
    fit::check_integrity(&encoded)?;

    std::fs::write(output, &encoded)?;

    println!(
        "{} files → {} ({} messages, {} bytes)",
        files.len().to_string().cyan(),
        output.green().bold(),
        all_messages.len(),
        encoded.len()
    );

    Ok(())
}

fn extract_timestamp(msg: &Message) -> Option<u64> {
    msg.field("timestamp")
        .and_then(|f| f.value.as_datetime())
        .map(|dt| dt.timestamp() as u64)
}
