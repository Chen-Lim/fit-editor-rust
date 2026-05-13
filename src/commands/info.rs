use std::collections::HashMap;
use std::path::Path;

use colored::Colorize;

use crate::error::CliError;

pub fn run(file: &str, verbose: bool) -> Result<(), CliError> {
    let path = Path::new(file);
    let bytes = std::fs::read(path)?;

    if !fit::is_fit(&bytes) {
        return Err(CliError::NotFit(file.to_string()));
    }

    let header = fit::FileHeader::parse(&bytes)
        .map_err(CliError::from)?;

    let (messages, errors) = fit::Decoder::builder(&bytes).build().read_all();

    // Count messages by type
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for msg in &messages {
        *counts.entry(msg.name).or_default() += 1;
    }

    // Sort by count descending
    let mut sorted: Vec<_> = counts.into_iter().collect();
    sorted.sort_by_key(|b| std::cmp::Reverse(b.1));

    let proto_major = header.protocol_version >> 4;
    let proto_minor = header.protocol_version & 0x0F;

    println!("{}", "File Information".bold());
    println!(
        "  {:<16} {}",
        "File:".dimmed(),
        path.file_name().unwrap_or_default().to_string_lossy()
    );
    println!(
        "  {:<16} {}.{} (0x{:02X})",
        "Protocol:".dimmed(),
        proto_major,
        proto_minor,
        header.protocol_version
    );
    println!(
        "  {:<16} {}.{}",
        "Profile:".dimmed(),
        header.profile_version / 100,
        header.profile_version % 100
    );
    println!(
        "  {:<16} {} bytes",
        "Data Size:".dimmed(),
        header.data_size
    );
    println!("  {:<16} {}", "Messages:".dimmed(), messages.len());

    println!("\n{}", "Message Counts".bold());
    for (name, count) in &sorted {
        println!("  {:<20} {}", name.cyan(), count);
    }

    if !errors.is_empty() && verbose {
        eprintln!(
            "\n{} {} decode warning(s)",
            "⚠".yellow(),
            errors.len()
        );
        for err in &errors {
            eprintln!("  {err:?}");
        }
    }

    Ok(())
}
