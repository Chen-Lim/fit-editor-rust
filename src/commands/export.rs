use std::collections::BTreeMap;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use colored::Colorize;
use fit::{Decoder, FieldKind, Message, Value};
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
        .map_err(CliError::from)?;

    let (messages, _errors) = Decoder::builder(&bytes).build().read_all();

    match format {
        ExportFormat::Json => export_json(&messages, &header, output, message_filter, pretty),
        ExportFormat::Csv => export_csv(&messages, output, message_filter),
        ExportFormat::Gpx => export_gpx(&messages, output),
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
            // Skip developer metadata — meaningless without developer fields
            .filter(|(_, msg)| !matches!(msg.name, "field_description" | "developer_data_id"))
            .map(|(idx, msg)| ExportMessage {
                index: idx,
                msg_type: msg.name.to_string(),
                fields: msg
                    .fields
                    .iter()
                    // Skip component-synthesized fields and developer fields
                    .filter(|f| match f.kind {
                        FieldKind::Standard { field_def_num } => field_def_num != 0,
                        FieldKind::Developer { .. } => false,
                    })
                    // Skip byte-array fields — JSON has no lossless byte representation
                    .filter(|f| !matches!(f.value, Value::Bytes(_)))
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
    let filter = message_filter.unwrap_or("record");
    let records: Vec<&fit::Message> = messages.iter().filter(|m| m.name == filter).collect();

    if records.is_empty() {
        eprintln!("No '{filter}' messages found");
        return Ok(());
    }

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

    wtr.write_record(&field_names)?;

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

fn export_gpx(messages: &[Message], output: Option<&str>) -> Result<(), CliError> {
    let records: Vec<&Message> = messages.iter().filter(|m| m.name == "record").collect();

    if records.is_empty() {
        eprintln!("{}", "No record messages found".yellow());
        return Ok(());
    }

    let mut gpx = String::new();
    gpx.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    gpx.push_str("<gpx version=\"1.1\" creator=\"fit-editor\"\n");
    gpx.push_str("     xmlns=\"http://www.topografix.com/GPX/1/1\"\n");
    gpx.push_str("     xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"\n");
    gpx.push_str("     xsi:schemaLocation=\"http://www.topografix.com/GPX/1/1 http://www.topografix.com/GPX/1/1/gpx.xsd\">\n");
    gpx.push_str("  <trk>\n");
    gpx.push_str("    <name>Activity</name>\n");
    gpx.push_str("    <trkseg>\n");

    for record in &records {
        let lat = record
            .field("position_lat")
            .and_then(|f| f.value.as_f64())
            .map(semicircles_to_degrees);

        let lon = record
            .field("position_long")
            .and_then(|f| f.value.as_f64())
            .map(semicircles_to_degrees);

        let ele = record
            .field("enhanced_altitude")
            .or_else(|| record.field("altitude"))
            .and_then(|f| f.value.as_f64());

        let time = record
            .field("timestamp")
            .and_then(|f| match &f.value {
                Value::DateTime(dt) => Some(dt.to_rfc3339()),
                _ => None,
            });

        let has_coords = lat.is_some() && lon.is_some();
        let lat = lat.unwrap_or(0.0);
        let lon = lon.unwrap_or(0.0);

        if !has_coords && ele.is_none() && time.is_none() {
            continue;
        }

        gpx.push_str(&format!(
            "      <trkpt lat=\"{:.7}\" lon=\"{:.7}\">\n",
            lat, lon
        ));

        if let Some(e) = ele {
            gpx.push_str(&format!("        <ele>{e:.1}</ele>\n"));
        }
        if let Some(t) = time {
            gpx.push_str(&format!("        <time>{t}</time>\n"));
        }

        // Extensions: HR, cadence, speed, power
        let mut has_extensions = false;
        let mut ext = String::new();

        if let Some(hr) = record.field("heart_rate").and_then(|f| f.value.as_f64()) {
            ext.push_str(&format!(
                "          <gpxtpx:TrackPointExtension>\n\
                 <gpxtpx:hr>{:.0}</gpxtpx:hr>\n\
                 </gpxtpx:TrackPointExtension>\n",
                hr
            ));
            has_extensions = true;
        }

        if has_extensions {
            gpx.push_str("        <extensions>\n");
            gpx.push_str("          <gpxtpx:TrackPointExtension xmlns:gpxtpx=\"http://www.garmin.com/xmlschemas/TrackPointExtension/v1\">\n");
            gpx.push_str(&ext);
            gpx.push_str("        </extensions>\n");
        }

        gpx.push_str("      </trkpt>\n");
    }

    gpx.push_str("    </trkseg>\n");
    gpx.push_str("  </trk>\n");
    gpx.push_str("</gpx>\n");

    if let Some(out_path) = output {
        std::fs::write(out_path, &gpx)?;
        println!("{} {}", "Exported".green(), out_path);
    } else {
        print!("{gpx}");
    }

    Ok(())
}

/// Convert semicircles to degrees (lat/lon in FIT)
fn semicircles_to_degrees(semicircles: f64) -> f64 {
    semicircles * (180.0 / 2_147_483_648.0)
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
