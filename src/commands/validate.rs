use std::path::Path;

use colored::Colorize;

use crate::error::CliError;

pub fn run(file: &str, _verbose: bool) -> Result<(), CliError> {
    let path = Path::new(file);
    let bytes = std::fs::read(path)?;

    if !fit::is_fit(&bytes) {
        eprintln!("{}: {}", file.red().bold(), "not a FIT file".red());
        return Err(CliError::NotFit(file.to_string()));
    }

    match fit::check_integrity(&bytes) {
        Ok(()) => {
            println!("{}: {}", file.green().bold(), "valid".green());
            Ok(())
        }
        Err(e) => {
            let cli_err = CliError::from(e);
            eprintln!("{}: {}", file.red().bold(), cli_err);
            Err(cli_err)
        }
    }
}
