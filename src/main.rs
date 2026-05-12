mod cli;
mod commands;
mod error;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), error::CliError> {
    match &cli.command {
        Command::Validate { file } => commands::validate::run(file, cli.verbose),

        Command::Info { file } => commands::info::run(file, cli.verbose),

        Command::Dump {
            file,
            message,
            field,
            limit,
            raw,
            compact,
        } => commands::dump::run(
            file,
            message.as_deref(),
            field.as_deref(),
            *limit,
            *raw,
            *compact,
            cli.verbose,
        ),

        Command::Export {
            file,
            format,
            output,
            message,
            pretty,
            compact,
        } => commands::export::run(
            file,
            format,
            output.as_deref(),
            message.as_deref(),
            *pretty,
            *compact,
        ),

        Command::Encode { file, output } => commands::encode::run(file, output),

        // Phase 1 stubs
        Command::Edit { .. }
        | Command::Merge { .. }
        | Command::Split { .. }
        | Command::Diff { .. }
        | Command::Summary { .. }
        | Command::Hexdump { .. }
        | Command::Batch { .. } => {
            eprintln!("not yet implemented");
            std::process::exit(2);
        }
    }
}
