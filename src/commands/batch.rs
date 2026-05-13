use std::process::Command;

use colored::Colorize;
use glob::glob;

use crate::error::CliError;

pub fn run(pattern: &str, cmd_args: &[String]) -> Result<(), CliError> {
    if cmd_args.is_empty() {
        return Err(CliError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "batch requires a command after --",
        )));
    }

    let entries: Vec<_> = glob(pattern)
        .map_err(|e| CliError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e)))?
        .filter_map(Result::ok)
        .collect();

    if entries.is_empty() {
        eprintln!("{}", "No files matched pattern".yellow());
        return Ok(());
    }

    println!(
        "{} files matched '{}'",
        entries.len().to_string().cyan(),
        pattern
    );
    println!();

    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for (i, path) in entries.iter().enumerate() {
        let path_str = path.to_string_lossy();
        print!(
            "[{}/{}] {} ... ",
            i + 1,
            entries.len(),
            path_str.dimmed()
        );

        // Build command: replace placeholder or append file path
        let mut cmd = Command::new(&cmd_args[0]);
        if cmd_args.len() > 1 {
            cmd.args(&cmd_args[1..]);
        }
        cmd.arg(path.as_os_str());

        match cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    println!("{}", "ok".green());
                    succeeded += 1;
                    // Show stdout if non-empty
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if !stdout.trim().is_empty() {
                        for line in stdout.lines() {
                            println!("    {line}");
                        }
                    }
                } else {
                    println!(
                        "{} (exit {})",
                        "FAIL".red(),
                        output.status.code().unwrap_or(-1)
                    );
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stderr.trim().is_empty() {
                        for line in stderr.lines() {
                            eprintln!("    {line}");
                        }
                    }
                    failed += 1;
                }
            }
            Err(e) => {
                println!("{} ({e})", "ERROR".red());
                failed += 1;
            }
        }
    }

    println!();
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
        std::process::exit(1);
    }

    Ok(())
}
