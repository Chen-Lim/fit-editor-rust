use std::collections::HashMap;
use std::path::Path;

use colored::Colorize;
use tabled::builder::Builder;
use tabled::settings::Style;

use crate::error::{read_bounded, CliError, DEFAULT_MAX_FILE_SIZE};

pub fn run(file: &str, verbose: bool) -> Result<(), CliError> {
    let path = Path::new(file);
    let bytes = read_bounded(path, DEFAULT_MAX_FILE_SIZE)?;

    if !fit::is_fit(&bytes) {
        return Err(CliError::NotFit(file.to_string()));
    }

    let header = fit::FileHeader::parse(&bytes).map_err(CliError::from)?;

    let (messages, errors) = fit::Decoder::builder(&bytes).build().read_all();

    let mut counts: HashMap<&str, usize> = HashMap::new();
    for msg in &messages {
        *counts.entry(msg.name).or_default() += 1;
    }

    let mut sorted: Vec<_> = counts.into_iter().collect();
    sorted.sort_by_key(|b| std::cmp::Reverse(b.1));

    let proto_major = header.protocol_version >> 4;
    let proto_minor = header.protocol_version & 0x0F;

    println!("{}", "File Information".bold());
    let mut info = Builder::default();
    info.push_record([
        "File".to_string(),
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
    ]);
    info.push_record([
        "Protocol".to_string(),
        format!(
            "{}.{} (0x{:02X})",
            proto_major, proto_minor, header.protocol_version
        ),
    ]);
    info.push_record([
        "Profile".to_string(),
        format!(
            "{}.{}",
            header.profile_version / 100,
            header.profile_version % 100
        ),
    ]);
    info.push_record([
        "Data Size".to_string(),
        format!("{} bytes", header.data_size),
    ]);
    info.push_record(["Messages".to_string(), messages.len().to_string()]);
    let mut info_tbl = info.build();
    info_tbl.with(Style::sharp());
    println!("{info_tbl}");

    println!("\n{}", "Message Counts".bold());
    let mut counts_b = Builder::default();
    counts_b.push_record(["Name", "Count"]);
    for (name, count) in &sorted {
        counts_b.push_record([name.to_string(), count.to_string()]);
    }
    let mut counts_tbl = counts_b.build();
    counts_tbl.with(Style::sharp());
    println!("{counts_tbl}");

    if !errors.is_empty() && verbose {
        eprintln!("\n{} {} decode warning(s)", "⚠".yellow(), errors.len());
        for err in &errors {
            eprintln!("  {err:?}");
        }
    }

    Ok(())
}
