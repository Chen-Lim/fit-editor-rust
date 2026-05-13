use std::path::Path;

use colored::Colorize;
use fit::{Decoder, Message, Value};
use tabled::builder::Builder;
use tabled::settings::Style;

use crate::error::CliError;

pub fn run(file: &str) -> Result<(), CliError> {
    let path = Path::new(file);
    let bytes = std::fs::read(path)?;

    if !fit::is_fit(&bytes) {
        return Err(CliError::NotFit(file.to_string()));
    }

    let (messages, _errors) = Decoder::builder(&bytes).build().read_all();

    let session = messages.iter().find(|m| m.name == "session");
    let file_id = messages.iter().find(|m| m.name == "file_id");
    let records: Vec<&Message> = messages.iter().filter(|m| m.name == "record").collect();

    let mut rows: Vec<(String, String)> = Vec::new();

    let sport = session
        .and_then(|m| m.field("sport"))
        .and_then(|f| f.value.as_str())
        .unwrap_or("unknown");
    rows.push(("Sport".into(), sport.to_string()));

    let start_time = session
        .and_then(|m| m.field("start_time"))
        .or_else(|| file_id.and_then(|m| m.field("time_created")))
        .map(|f| format_value(&f.value))
        .unwrap_or_else(|| "?".into());
    rows.push(("Start Time".into(), start_time));

    let timer_time = session
        .and_then(|m| m.field("total_timer_time"))
        .and_then(|f| f.value.as_f64())
        .unwrap_or(0.0);
    let elapsed_time = session
        .and_then(|m| m.field("total_elapsed_time"))
        .and_then(|f| f.value.as_f64())
        .unwrap_or(timer_time);
    rows.push((
        "Duration".into(),
        format!(
            "{} (timer) / {} (elapsed)",
            format_duration(timer_time),
            format_duration(elapsed_time)
        ),
    ));

    let distance = session
        .and_then(|m| m.field("total_distance"))
        .and_then(|f| f.value.as_f64())
        .unwrap_or(0.0);
    if distance > 0.0 {
        let s = if distance >= 1000.0 {
            format!("{:.2} km", distance / 1000.0)
        } else {
            format!("{distance:.0} m")
        };
        rows.push(("Distance".into(), s));
    }

    let avg_speed = session
        .and_then(|m| m.field("enhanced_avg_speed").or_else(|| m.field("avg_speed")))
        .and_then(|f| f.value.as_f64());
    let max_speed = session
        .and_then(|m| m.field("enhanced_max_speed").or_else(|| m.field("max_speed")))
        .and_then(|f| f.value.as_f64());
    if let Some(s) = avg_speed {
        rows.push((
            "Avg Speed".into(),
            format!("{s:.2} m/s ({:.1} km/h)", s * 3.6),
        ));
    }
    if let Some(s) = max_speed {
        rows.push((
            "Max Speed".into(),
            format!("{s:.2} m/s ({:.1} km/h)", s * 3.6),
        ));
    }

    let avg_hr = session
        .and_then(|m| m.field("avg_heart_rate"))
        .and_then(|f| f.value.as_f64());
    let max_hr = session
        .and_then(|m| m.field("max_heart_rate"))
        .and_then(|f| f.value.as_f64());
    if avg_hr.is_some() || max_hr.is_some() {
        let avg_str = avg_hr.map(|v| format!("{v:.0}")).unwrap_or_else(|| "?".into());
        let max_str = max_hr.map(|v| format!("{v:.0}")).unwrap_or_else(|| "?".into());
        rows.push((
            "Heart Rate".into(),
            format!("{avg_str} bpm (max: {max_str})"),
        ));
    }

    let calories = session
        .and_then(|m| m.field("total_calories"))
        .and_then(|f| f.value.as_f64());
    if let Some(cal) = calories {
        rows.push(("Calories".into(), format!("{cal:.0} kcal")));
    }

    let ascent = session
        .and_then(|m| m.field("total_ascent"))
        .and_then(|f| f.value.as_f64());
    let descent = session
        .and_then(|m| m.field("total_descent"))
        .and_then(|f| f.value.as_f64());
    if ascent.is_some() || descent.is_some() {
        rows.push((
            "Elevation".into(),
            format!(
                "{:.0} m ↑ / {:.0} m ↓",
                ascent.unwrap_or(0.0),
                descent.unwrap_or(0.0)
            ),
        ));
    }

    let avg_cadence = session
        .and_then(|m| m.field("avg_cadence"))
        .and_then(|f| f.value.as_f64());
    if let Some(cad) = avg_cadence {
        rows.push(("Avg Cadence".into(), format!("{cad:.0} rpm")));
    }

    println!("{}", "Activity Summary".bold());
    let mut b = Builder::default();
    for (k, v) in &rows {
        b.push_record([k.clone(), v.clone()]);
    }
    let mut t = b.build();
    t.with(Style::sharp());
    println!("{t}");

    if !records.is_empty() {
        println!("\n{}", "Records".bold());
        let mut rb = Builder::default();
        rb.push_record(["Data Points".to_string(), records.len().to_string()]);

        let hr_values: Vec<f64> = records
            .iter()
            .filter_map(|r| r.field("heart_rate").and_then(|f| f.value.as_f64()))
            .collect();

        if !hr_values.is_empty() {
            let min_hr = hr_values.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_hr = hr_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            rb.push_record([
                "HR Range".to_string(),
                format!("{min_hr:.0} - {max_hr:.0} bpm"),
            ]);
        }

        let mut rt = rb.build();
        rt.with(Style::sharp());
        println!("{rt}");
    }

    Ok(())
}

fn format_duration(secs: f64) -> String {
    let total = secs as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

fn format_value(value: &Value) -> String {
    match value {
        Value::DateTime(dt) => dt.to_rfc3339(),
        Value::Enum(name) => name.to_string(),
        Value::Float(v) => format!("{v:.4}"),
        Value::UInt(v) => v.to_string(),
        Value::SInt(v) => v.to_string(),
        Value::String(s) => s.clone(),
        _ => format!("{value:?}"),
    }
}
