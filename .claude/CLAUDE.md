# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`fit-editor` is a pure-Rust CLI tool for viewing, editing, and converting Garmin FIT files. Published on crates.io, depends on `fit-sdk-rust`.

## Build & Test Commands

```bash
cargo build                    # debug build
cargo build --release          # release build (significantly faster for large files)
cargo test                     # run all tests
cargo test -- --nocapture      # run tests with stdout visible
cargo test export_json         # run a single test by name substring
cargo clippy                   # lint
cargo run -- <args>            # run debug binary, e.g. cargo run -- info Activity.fit
```

Integration tests use a fixture file at `tests/fixtures/Activity.fit`.

## Architecture

```
src/
  cli.rs            # clap derive definitions — all subcommands and global flags
  main.rs           # entry point, CLI parse → TTY/color setup → dispatch to commands
  error.rs          # CliError enum with From impls for io/serde/csv/fit errors
  commands/         # one module per subcommand, each exports a pub fn run(...)
    mod.rs          # re-exports all command modules
    validate.rs     # CRC + signature check via fit::check_integrity()
    info.rs         # header metadata + message count tables (tabled)
    dump.rs         # human-readable message dump with filtering
    export.rs       # JSON/CSV/GPX export
    encode.rs       # JSON → FIT round-trip
    edit.rs         # field mutation, message removal → re-encode
    merge.rs        # timestamp-ordered merge of multiple FIT files
    split.rs        # split at timestamp or message index
    diff.rs         # structured comparison of two FIT files
    summary.rs      # activity stats extraction (tabled tables)
    hexdump.rs      # xxd-style hex dump with optional annotations
    batch.rs        # glob-based batch processing with indicatif progress bar
```

Each command module is self-contained: it receives parsed CLI args, reads FIT files via the SDK, and writes output. There is no shared service layer — commands call `fit::Decoder` and `fit::Encoder` directly.

## Key SDK Integration

`fit-sdk-rust` is a crates.io dependency. Core API:

```rust
// Decode: bytes → messages
let bytes = std::fs::read(path)?;
let (messages, errors) = fit::Decoder::builder(&bytes).build().read_all();

// Encode: messages → bytes
let encoded: Vec<u8> = fit::Encoder::new().encode(&messages)?;

// Integrity check
fit::is_fit(&bytes) -> bool
fit::check_integrity(&bytes) -> Result<(), FitError>
```

Messages are `fit::Message` with fields accessed via `.field("name")` returning `Option<&Field>`. Values are `fit::Value` enum variants (DateTime, Float, String, Enum, etc.).

## Conventions

- One command module per file, each with `pub fn run(...) -> Result<(), CliError>`
- Global flags (`-v`, `-q`, `--no-color`) are in `cli.rs` on the `Cli` struct, accessed as `cli.verbose` etc.
- `build.rs` generates man pages via `clap_mangen` at compile time (output in `target/<profile>/build/fit-editor-*/out/man/`)
- Tests use `assert_cmd` + `predicates` for CLI integration testing; fixtures at `tests/fixtures/`
- Output formatting: `tabled` for tables, `colored` for TTY colors (auto-disabled when not TTY or `--no-color`), `indicatif` for progress bars
