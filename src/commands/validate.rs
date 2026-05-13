use std::path::Path;

use colored::Colorize;
use fit::Decoder;

use crate::error::{read_bounded, CliError, DEFAULT_MAX_FILE_SIZE};

pub fn run(file: &str, verbose: bool) -> Result<(), CliError> {
    let path = Path::new(file);
    let bytes = read_bounded(path, DEFAULT_MAX_FILE_SIZE)?;

    if !fit::is_fit(&bytes) {
        eprintln!("{}: {}", file.red().bold(), "not a FIT file".red());
        return Err(CliError::NotFit(file.to_string()));
    }

    // CRC + signature check
    if let Err(e) = fit::check_integrity(&bytes) {
        let cli_err = CliError::from(e);
        eprintln!("{}: {}", file.red().bold(), cli_err);
        return Err(cli_err);
    }

    // Decoder-level check
    let (_messages, errors) = Decoder::builder(&bytes).build().read_all();

    if errors.is_empty() {
        println!("{}: {}", file.green().bold(), "valid".green());
        Ok(())
    } else {
        eprintln!(
            "{}: CRC ok but {} decoder error(s)",
            file.yellow().bold(),
            errors.len()
        );
        if verbose {
            for err in &errors {
                eprintln!("  {err:?}");
            }
        }
        Err(CliError::DecodeErrors(errors.len()))
    }
}
