mod cli;
mod commands;
mod error;

use std::io::{self, IsTerminal};

use clap::{CommandFactory, Parser};
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();

    let stdout_tty = io::stdout().is_terminal();
    if cli.no_color || !stdout_tty {
        colored::control::set_override(false);
    }

    if let Err(e) = run(cli, stdout_tty) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run(cli: Cli, stdout_tty: bool) -> Result<(), error::CliError> {
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

        Command::Edit {
            file,
            output,
            set,
            remove_message,
        } => commands::edit::run(file, output, set, remove_message),

        Command::Merge { files, output } => commands::merge::run(files, output),

        Command::Split {
            file,
            output,
            at,
            at_index,
        } => commands::split::run(file, output, at.as_deref(), *at_index),

        Command::Diff {
            file1,
            file2,
            ignore_timestamps,
            message,
        } => commands::diff::run(file1, file2, *ignore_timestamps, message.as_deref()),

        Command::Summary { file } => commands::summary::run(file),

        Command::Hexdump {
            file,
            bytes,
            skip_header,
            annotate,
        } => commands::hexdump::run(file, *bytes, *skip_header, *annotate),

        Command::Batch { glob, command } => commands::batch::run(glob, command, stdout_tty),

        Command::Completion { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(*shell, &mut cmd, "fit-editor", &mut io::stdout());
            Ok(())
        }
    }
}
