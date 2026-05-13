use std::path::Path;

use colored::Colorize;
use fit::{Decoder, Encoder, Message};

use crate::error::CliError;

pub fn run(files: &[String], output: &str) -> Result<(), CliError> {
    if files.len() < 2 {
        return Err(CliError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "merge requires at least 2 files",
        )));
    }

    let mut all_messages: Vec<Message> = Vec::new();

    for file in files {
        let path = Path::new(file);
        let bytes = std::fs::read(path)?;

        if !fit::is_fit(&bytes) {
            return Err(CliError::NotFit(file.clone()));
        }

        let (messages, _errors) = Decoder::builder(&bytes).build().read_all();
        all_messages.extend(messages);
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
    msg.field("timestamp").and_then(|f| f.value.as_u64())
}
