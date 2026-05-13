use std::path::Path;

use colored::Colorize;
use fit::{Decoder, Message, Value};

use crate::error::CliError;

pub fn run(file: &str) -> Result<(), CliError> {
    let path = Path::new(file);
    let bytes = std::fs::read(path)?;

    if !fit::is_fit(&bytes) {
        return Err(CliError::NotFit(file.to_string()));
    }

    let (messages, _errors) = Decoder::builder(&bytes).build().read_all();

    // Find session message for summary data
    let session = messages.iter().find(|m| m.name == "session");

    // Find file_id for metadata
    let file_id = messages.iter().find(|m| m.name == "file_id");

    // Find records for per-point stats
    let records: Vec<&Message> = messages.iter().filter(|m| m.name == "record").collect();

    println!("{}", "Activity Summary".bold());
    println!("{}", "─".repeat(40).dimmed());

    // Sport
    let sport = session
        .and_then(|m| m.field("sport"))
        .and_then(|f| f.value.as_str())
        .unwrap_or("unknown");
    println!("  {:<16} {}", "Sport:".dimmed(), sport.cyan());

    // Start time
    let start_time = session
        .and_then(|m| m.field("start_time"))
        .or_else(|| file_id.and_then(|m| m.field("time_created")))
        .map(|f| format_value(&f.value))
        .unwrap_or_else(|| "?".into());
    println!("  {:<16} {}", "Start Time:".dimmed(), start_time);

    // Duration
    let timer_time = session
        .and_then(|m| m.field("total_timer_time"))
        .and_then(|f| f.value.as_f64())
        .unwrap_or(0.0);
    let elapsed_time = session
        .and_then(|m| m.field("total_elapsed_time"))
        .and_then(|f| f.value.as_f64())
        .unwrap_or(timer_time);

    println!(
        "  {:<16} {} (timer) / {} (elapsed)",
        "Duration:".dimmed(),
        format_duration(timer_time),
        format_duration(elapsed_time),
    );

    // Distance
    let distance = session
        .and_then(|m| m.field("total_distance"))
        .and_then(|f| f.value.as_f64())
        .unwrap_or(0.0);
    if distance > 0.0 {
        if distance >= 1000.0 {
            println!(
                "  {:<16} {:.2} km",
                "Distance:".dimmed(),
                distance / 1000.0
            );
        } else {
            println!("  {:<16} {:.0} m", "Distance:".dimmed(), distance);
        }
    }

    // Speed
    let avg_speed = session
        .and_then(|m| m.field("enhanced_avg_speed").or_else(|| m.field("avg_speed")))
        .and_then(|f| f.value.as_f64());
    let max_speed = session
        .and_then(|m| m.field("enhanced_max_speed").or_else(|| m.field("max_speed")))
        .and_then(|f| f.value.as_f64());

    if let Some(s) = avg_speed {
        println!(
            "  {:<16} {:.2} m/s ({:.1} km/h)",
            "Avg Speed:".dimmed(),
            s,
            s * 3.6
        );
    }
    if let Some(s) = max_speed {
        println!(
            "  {:<16} {:.2} m/s ({:.1} km/h)",
            "Max Speed:".dimmed(),
            s,
            s * 3.6
        );
    }

    // Heart rate
    let avg_hr = session
        .and_then(|m| m.field("avg_heart_rate"))
        .and_then(|f| f.value.as_f64());
    let max_hr = session
        .and_then(|m| m.field("max_heart_rate"))
        .and_then(|f| f.value.as_f64());

    if avg_hr.is_some() || max_hr.is_some() {
        let avg_str = avg_hr.map(|v| format!("{v:.0}")).unwrap_or_else(|| "?".into());
        let max_str = max_hr.map(|v| format!("{v:.0}")).unwrap_or_else(|| "?".into());
        println!(
            "  {:<16} {} bpm (max: {})",
            "Heart Rate:".dimmed(),
            avg_str,
            max_str,
        );
    }

    // Calories
    let calories = session
        .and_then(|m| m.field("total_calories"))
        .and_then(|f| f.value.as_f64());
    if let Some(cal) = calories {
        println!("  {:<16} {:.0} kcal", "Calories:".dimmed(), cal);
    }

    // Ascent / Descent
    let ascent = session
        .and_then(|m| m.field("total_ascent"))
        .and_then(|f| f.value.as_f64());
    let descent = session
        .and_then(|m| m.field("total_descent"))
        .and_then(|f| f.value.as_f64());

    if ascent.is_some() || descent.is_some() {
        println!(
            "  {:<16} {:.0} m ↑ / {:.0} m ↓",
            "Elevation:".dimmed(),
            ascent.unwrap_or(0.0),
            descent.unwrap_or(0.0),
        );
    }

    // Cadence
    let avg_cadence = session
        .and_then(|m| m.field("avg_cadence"))
        .and_then(|f| f.value.as_f64());
    if let Some(cad) = avg_cadence {
        println!("  {:<16} {:.0} rpm", "Avg Cadence:".dimmed(), cad);
    }

    // Record stats
    if !records.is_empty() {
        println!();
        println!("{}", "Records".bold());
        println!("  {:<16} {}", "Data Points:".dimmed(), records.len());

        let hr_values: Vec<f64> = records
            .iter()
            .filter_map(|r| r.field("heart_rate").and_then(|f| f.value.as_f64()))
            .collect();

        if !hr_values.is_empty() {
            let min_hr = hr_values.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_hr = hr_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            println!(
                "  {:<16} {:.0} - {:.0} bpm",
                "HR Range:".dimmed(),
                min_hr,
                max_hr,
            );
        }
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
