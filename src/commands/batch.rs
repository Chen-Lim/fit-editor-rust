use std::process::Command;

use colored::Colorize;
use glob::glob;
use indicatif::{ProgressBar, ProgressStyle};

use crate::error::CliError;

pub fn run(pattern: &str, cmd_args: &[String], stdout_tty: bool) -> Result<(), CliError> {
    if cmd_args.is_empty() {
        return Err(CliError::BadUsage(
            "batch requires a command after --".into(),
        ));
    }

    let entries: Vec<_> = glob(pattern)
        .map_err(|e| CliError::BadUsage(format!("invalid glob pattern: {e}")))?
        .filter_map(Result::ok)
        .collect();

    if entries.is_empty() {
        eprintln!("{}", "No files matched pattern".yellow());
        return Ok(());
    }

    let bar = if stdout_tty {
        let b = ProgressBar::new(entries.len() as u64);
        b.set_style(
            ProgressStyle::with_template(
                "[{bar:40.cyan/blue}] {pos}/{len} {wide_msg}",
            )
            .expect("valid progress bar template")
            .progress_chars("=>-"),
        );
        b
    } else {
        ProgressBar::hidden()
    };

    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for path in &entries {
        let path_str = path.to_string_lossy().to_string();
        bar.set_message(path_str.clone());

        let mut cmd = Command::new(&cmd_args[0]);
        if cmd_args.len() > 1 {
            cmd.args(&cmd_args[1..]);
        }
        cmd.arg(path.as_os_str());

        match cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    succeeded += 1;
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if !stdout.trim().is_empty() {
                        bar.println(format!("{} {path_str}", "ok".green()));
                        for line in stdout.lines() {
                            bar.println(format!("    {line}"));
                        }
                    } else {
                        bar.println(format!("{} {path_str}", "ok".green()));
                    }
                } else {
                    failed += 1;
                    bar.println(format!(
                        "{} {path_str} (exit {})",
                        "FAIL".red(),
                        output.status.code().unwrap_or(-1)
                    ));
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stderr.trim().is_empty() {
                        for line in stderr.lines() {
                            bar.println(format!("    {line}"));
                        }
                    }
                }
            }
            Err(e) => {
                failed += 1;
                bar.println(format!("{} {path_str} ({e})", "ERROR".red()));
            }
        }

        bar.inc(1);
    }

    bar.finish_and_clear();

    println!(
        "{} succeeded, {} failed",
        succeeded.to_string().green(),
        if failed > 0 {
            failed.to_string().red().to_string()
        } else {
            "0".to_string()
        }
    );

    if failed > 0 {
        return Err(CliError::BatchPartialFailure { succeeded, failed });
    }

    Ok(())
}
