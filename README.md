# fit-editor

Pure-Rust CLI tool for viewing, editing, and converting Garmin FIT files.

Built on top of [fit-sdk-rust](https://github.com/Chen-Lim/fit-sdk-rust), `fit-editor` lets you inspect, validate, edit, encode, merge, split, diff, and summarize `.fit` activity files from the command line — no Garmin SDK required.

## Features

- **View** — `validate`, `info`, `dump`, `hexdump`
- **Convert** — `export` to JSON / CSV / GPX
- **Edit** — `encode` JSON back to FIT, `edit` fields, remove messages
- **Combine** — `merge` files by timestamp, `split` at timestamp or index, `diff` two files
- **Analyze** — `summary` of activity stats, `batch` processing with progress bar
- **DX** — shell completions (bash/zsh/fish/powershell), man pages, `--no-color`, TTY-aware output

## Installation

```bash
git clone https://github.com/Chen-Lim/fit-editor-rust.git
cd fit-editor-rust
cargo install --path .
```

Requires Rust 1.75+ and a sibling clone of [fit-sdk-rust](https://github.com/Chen-Lim/fit-sdk-rust) (path dependency).

## Quick start

```bash
# Inspect a file
fit-editor info Activity.fit
fit-editor summary Activity.fit
fit-editor dump Activity.fit --message record --limit 5

# Export
fit-editor export Activity.fit -f json -o activity.json
fit-editor export Activity.fit -f csv  -o records.csv --message record
fit-editor export Activity.fit -f gpx  -o track.gpx

# Edit & encode
fit-editor edit Activity.fit --set session.total_distance=5000.0 -o modified.fit
fit-editor encode activity.json -o roundtrip.fit
fit-editor validate roundtrip.fit

# Combine
fit-editor merge morning.fit afternoon.fit -o full_day.fit
fit-editor split Activity.fit --at 2024-01-15T11:00:00Z -o part
fit-editor diff a.fit b.fit --ignore-timestamps

# Batch
fit-editor batch '*.fit' -- fit-editor validate
```

## Command reference

| Command | Purpose |
|---------|---------|
| `validate <file>` | CRC + signature check |
| `info <file>` | Header metadata + message counts |
| `dump <file>` | Human-readable message dump (filter by `--message`, `--field`, `--limit`) |
| `export <file> -f <fmt>` | Export to `json` / `csv` / `gpx` |
| `encode <json> -o <fit>` | JSON → FIT (round-trips with `export -f json`) |
| `edit <file> -o <out>` | `--set field.path=value`, `--remove-message <type>` |
| `merge <files...> -o <out>` | Timestamp-ordered merge |
| `split <file> -o <prefix>` | `--at <RFC3339>` or `--at-index <N>` |
| `diff <f1> <f2>` | Structured diff (`--ignore-timestamps`, `--message`) |
| `summary <file>` | Activity stats: sport, duration, distance, HR, etc. |
| `hexdump <file>` | xxd-style dump with `--annotate` for message boundaries |
| `batch <glob> -- <cmd>` | Run command on each match, progress bar in TTY |
| `completion <shell>` | Emit completion script (`bash`, `zsh`, `fish`, `powershell`, `elvish`) |

Global flags: `-v/--verbose`, `-q/--quiet`, `--no-color`.

## Shell completion

```bash
# zsh
fit-editor completion zsh > ~/.zsh/completions/_fit-editor

# bash
fit-editor completion bash > /etc/bash_completion.d/fit-editor

# fish
fit-editor completion fish > ~/.config/fish/completions/fit-editor.fish
```

## Man pages

`cargo build` generates man pages under `target/<profile>/build/fit-editor-*/out/man/`:

```bash
man -l $(find target -name 'fit-editor.1' | head -1)
```

## Project layout

```
src/
  cli.rs           # clap definitions
  main.rs          # dispatch
  error.rs         # CliError
  commands/        # one module per subcommand
docs/              # PRD, architecture, format notes, roadmap
tests/             # phase2.rs, phase4.rs (integration tests)
build.rs           # clap_mangen man-page generation
```

See [docs/Rroadmap.md](docs/Rroadmap.md) for the phased plan. Phase 0–4 (code side) are complete; Phase 4.5 (CI release, Homebrew, crates.io publish) is upcoming.

## License

[GPL-3.0](LICENSE)
