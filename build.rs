use std::env;
use std::fs;
use std::path::PathBuf;

use clap::CommandFactory;
use clap_mangen::Man;

#[path = "src/cli.rs"]
mod cli;

fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=src/cli.rs");
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"));
    let man_dir = out_dir.join("man");
    fs::create_dir_all(&man_dir)?;

    let cmd = cli::Cli::command();
    let bin_name = cmd.get_name().to_string();

    let mut top = Vec::new();
    Man::new(cmd.clone()).render(&mut top)?;
    fs::write(man_dir.join(format!("{bin_name}.1")), top)?;

    for sub in cmd.get_subcommands() {
        if sub.is_hide_set() {
            continue;
        }
        let file_name = format!("{}-{}.1", bin_name, sub.get_name());
        let mut buf = Vec::new();
        Man::new(sub.clone()).render(&mut buf)?;
        fs::write(man_dir.join(file_name), buf)?;
    }

    Ok(())
}
