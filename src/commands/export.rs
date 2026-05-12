use std::collections::BTreeMap;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use fit::{Decoder, Value};
use serde::Serialize;

use crate::cli::ExportFormat;
use crate::error::CliError;

#[derive(Serialize)]
struct ExportFile {
    file_header: FileHeaderInfo,
    messages: Vec<ExportMessage>,
}

#[derive(Serialize)]
struct FileHeaderInfo {
    protocol_version: String,
    profile_version: u16,
    data_size: u32,
}

#[derive(Serialize)]
struct ExportMessage {
    index: usize,
    #[serde(rename = "type")]
    msg_type: String,
    fields: BTreeMap<String, serde_json::Value>,
}

pub fn run(
    file: &str,
    format: &ExportFormat,
    output: Option<&str>,
    message_filter: Option<&str>,
    pretty: bool,
    _compact: bool,
) -> Result<(), CliError> {
    let path = Path::new(file);
    let bytes = std::fs::read(path)?;

    if !fit::is_fit(&bytes) {
        return Err(CliError::NotFit(file.to_string()));
    }

    let header = fit::FileHeader::parse(&bytes)
        .map_err(|e| CliError::from(fit::FitError::from(e)))?;

    let (messages, _errors) = Decoder::builder(&bytes).build().read_all();

    match format {
        ExportFormat::Json => export_json(&messages, &header, output, message_filter, pretty),
        ExportFormat::Csv => export_csv(&messages, output, message_filter),
        ExportFormat::Gpx => {
            eprintln!("GPX export not yet implemented");
            Err(CliError::Io(io::Error::new(
                io::ErrorKind::Unsupported,
                "GPX export not yet implemented",
            )))
        }
    }
}

fn export_json(
    messages: &[fit::Message],
    header: &fit::FileHeader,
    output: Option<&str>,
    message_filter: Option<&str>,
    pretty: bool,
) -> Result<(), CliError> {
    let export = ExportFile {
        file_header: FileHeaderInfo {
            protocol_version: format!(
                "{}.{}",
                header.protocol_version >> 4,
                header.protocol_version & 0x0F
            ),
            profile_version: header.profile_version,
            data_size: header.data_size,
        },
        messages: messages
            .iter()
            .enumerate()
            .filter(|(_, msg)| message_filter.map_or(true, |f| msg.name == f))
            .map(|(idx, msg)| ExportMessage {
                index: idx,
                msg_type: msg.name.to_string(),
                fields: msg
                    .fields
                    .iter()
                    .map(|f| (f.name.clone(), value_to_json(&f.value)))
                    .collect(),
            })
            .collect(),
    };

    let json = if pretty {
        serde_json::to_string_pretty(&export)?
    } else {
        serde_json::to_string(&export)?
    };

    if let Some(out_path) = output {
        let mut w = BufWriter::new(std::fs::File::create(out_path)?);
        w.write_all(json.as_bytes())?;
        w.write_all(b"\n")?;
    } else {
        println!("{json}");
    }
    Ok(())
}

fn export_csv(
    messages: &[fit::Message],
    output: Option<&str>,
    message_filter: Option<&str>,
) -> Result<(), CliError> {
    // Default to record messages for CSV
    let filter = message_filter.unwrap_or("record");
    let records: Vec<&fit::Message> = messages.iter().filter(|m| m.name == filter).collect();

    if records.is_empty() {
        eprintln!("No '{filter}' messages found");
        return Ok(());
    }

    // Collect all unique field names across all matching messages
    let mut field_names: Vec<String> = Vec::new();
    for msg in &records {
        for f in &msg.fields {
            if !field_names.contains(&f.name) {
                field_names.push(f.name.clone());
            }
        }
    }

    let mut wtr: csv::Writer<Box<dyn Write>> = if let Some(out_path) = output {
        csv::Writer::from_writer(Box::new(BufWriter::new(std::fs::File::create(out_path)?)))
    } else {
        csv::Writer::from_writer(Box::new(io::stdout()))
    };

    // Header
    wtr.write_record(&field_names)?;

    // Rows
    for msg in &records {
        let row: Vec<String> = field_names
            .iter()
            .map(|name| {
                msg.field(name)
                    .map(|f| value_to_csv_string(&f.value))
                    .unwrap_or_default()
            })
            .collect();
        wtr.write_record(&row)?;
    }

    wtr.flush()?;
    Ok(())
}

fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Invalid => serde_json::Value::Null,
        Value::SInt(v) => serde_json::json!(v),
        Value::UInt(v) => serde_json::json!(v),
        Value::Float(v) => serde_json::json!(v),
        Value::String(s) => serde_json::json!(s),
        Value::Bytes(b) => serde_json::json!(format!("<{} bytes>", b.len())),
        Value::Bool(b) => serde_json::json!(b),
        Value::Enum(name) => serde_json::json!(name),
        Value::DateTime(dt) => serde_json::json!(dt.to_rfc3339()),
        Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(value_to_json).collect())
        }
    }
}

fn value_to_csv_string(value: &Value) -> String {
    match value {
        Value::Invalid => String::new(),
        Value::SInt(v) => v.to_string(),
        Value::UInt(v) => v.to_string(),
        Value::Float(v) => format!("{v:.6}"),
        Value::String(s) => s.clone(),
        Value::Bytes(b) => format!("<{} bytes>", b.len()),
        Value::Bool(b) => b.to_string(),
        Value::Enum(name) => name.to_string(),
        Value::DateTime(dt) => dt.to_rfc3339(),
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(value_to_csv_string).collect();
            items.join(";")
        }
    }
}
