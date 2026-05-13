---
name: fit-editor
description: Inspect, edit, convert, and analyze Garmin FIT activity files via the `fit-editor` CLI. Use this skill whenever the user mentions FIT files, .fit extensions, Garmin activity data, sport tracker exports, workout files from Garmin/Wahoo/Coros/Suunto, or asks to validate / dump / convert / merge / split / diff / summarize / hexdump / batch-process such files — even if they don't name the tool. Also use when the user asks to convert activity data to JSON / CSV / GPX, change session totals, strip messages, or round-trip a FIT file through JSON.
---

# fit-editor

`fit-editor` is a pure-Rust CLI for the Garmin FIT (Flexible & Interoperable Data Transfer) format. Use it for any task that touches `.fit` files — viewing, editing, converting, combining, comparing, summarizing, or batch processing.

## When to reach for this skill

Trigger whenever the user mentions any of:

- A `.fit` file or "FIT file" by name (including upper/lowercase variants).
- Garmin Connect exports, Garmin Edge / Forerunner / Fenix watches, or activity files from Wahoo / Coros / Suunto / Polar — they all use the FIT format.
- Tasks like "validate this activity", "extract heart rate", "convert to GPX/CSV/JSON", "merge two rides", "split an activity at X", "compare two workouts", "show me the summary".
- Anything about CRC checks, FIT record messages, session totals, or "round-trip a FIT file".

Don't trigger for TCX / GPX-only workflows that never touch FIT.

## Prerequisites

The `fit-editor` binary must be on PATH. Users typically install it by downloading a release binary from https://github.com/Chen-Lim/fit-editor-rust and placing it somewhere in PATH (`/usr/local/bin`, `~/.local/bin`, etc.).

Quick check:

```bash
command -v fit-editor && fit-editor --version
```

If missing, point the user at the GitHub releases page. Do **not** try to `cargo install` it unless the user explicitly wants a source build — distribution is binary-first.

## Command map

| You want to… | Use |
|---|---|
| Confirm a file is a valid FIT file (CRC + signature) | `fit-editor validate <file>` |
| See header + per-message-type counts | `fit-editor info <file>` |
| Read messages as text (humans) | `fit-editor dump <file> [--message T] [--field F] [--limit N] [--raw] [--compact]` |
| Get machine-readable data | `fit-editor export <file> -f json\|csv\|gpx -o <out>` |
| Build a FIT file from JSON | `fit-editor encode <in.json> -o <out.fit>` |
| Modify fields in place | `fit-editor edit <file> --set path=value -o <out>` |
| Drop all messages of a type | `fit-editor edit <file> --remove-message <type> -o <out>` |
| Merge multiple activities by time | `fit-editor merge a.fit b.fit -o merged.fit` |
| Split at timestamp or message index | `fit-editor split <file> --at <RFC3339> -o <prefix>` / `--at-index <N>` |
| Compare two files | `fit-editor diff <f1> <f2> [--ignore-timestamps] [--message T]` |
| Activity stats (sport / distance / HR / …) | `fit-editor summary <file>` |
| Byte-level view, annotated | `fit-editor hexdump <file> [--annotate] [-n bytes]` |
| Run a command across many files | `fit-editor batch '<glob>' -- <cmd> [args...]` |

Global flags (available on every subcommand):

- `-v, --verbose` — show decode warnings (helpful for triaging "why is field X missing").
- `-q, --quiet` — suppress decoration.
- `--no-color` — strip ANSI. **Always pass this when capturing output programmatically** so downstream parsing isn't tripped up by escape codes. (`fit-editor` also auto-disables color when stdout isn't a TTY, but `--no-color` makes the intent explicit and survives shells that fake a TTY.)

## Picking output formats

- **JSON** is the canonical interchange format. `export -f json` produces output that `encode` can read back losslessly. Use this for any programmatic work — modifying values, slicing data, feeding another tool.
- **CSV** flattens `record` messages to rows. Good for spreadsheet / pandas analysis. Pass `--message record` to limit to track points; otherwise schema gets messy because messages have different fields.
- **GPX** emits only the GPS track (lat/long/elev/time + HR extension). Use for map tools, Strava-style uploads, or visualization. It is lossy — you lose laps, sessions, sensor data.

Default is JSON; specify `-f` to override.

## Common workflows

### Inspect an unknown file before touching it

```bash
fit-editor --no-color validate path/to/Activity.fit
fit-editor --no-color info path/to/Activity.fit
fit-editor --no-color summary path/to/Activity.fit
```

Three quick commands tell you: is the file intact, what's inside, and what activity it represents.

### Programmatic field extraction

When the user wants specific values (e.g., "what's the total distance"), prefer JSON export piped to `jq` over scraping `dump` text. It's stable and lossless:

```bash
fit-editor export Activity.fit -f json -o /tmp/a.json
jq '.messages[] | select(.type=="session") | .fields.total_distance' /tmp/a.json
```

### Edit a single field

The `--set` path syntax is `<message_type>.<field>=<value>` or `<message_type>[<index>].<field>=<value>`. Numbers parse as numbers; bare strings as strings.

```bash
fit-editor edit Activity.fit \
  --set session.total_distance=5000.0 \
  --set session.sport=cycling \
  -o modified.fit

fit-editor validate modified.fit   # always verify after editing
```

After any `edit` or `encode`, run `validate` to catch encoding bugs early.

### Round-trip via JSON

When the user wants a complex modification that `--set` can't express, go through JSON:

1. `fit-editor export in.fit -f json -o /tmp/a.json`
2. Edit `/tmp/a.json` with whatever logic is needed (jq, a Python script, manual edit).
3. `fit-editor encode /tmp/a.json -o out.fit`
4. `fit-editor validate out.fit`

This is the safest path for bulk or conditional changes.

### Merge / split

`merge` orders messages by timestamp and remaps `local_mesg_num` collisions — safe to throw multiple files at it.

`split --at <RFC3339>` needs an ISO-8601 timestamp (`2024-01-15T11:00:00Z`). For "split at the halfway point" use `--at-index <N>` with N from `fit-editor info` (≈ messages/2).

### Diff

`diff` works well for "did my edit change what I expected" sanity checks. Use `--ignore-timestamps` when comparing files where only metadata changed — otherwise every `record` looks different. Scope to one message type with `-m session` to compare only the summary.

### Batch

`batch '<glob>'` expands the glob, then runs `<cmd> <each_file>` per match. In a TTY it shows an indicatif progress bar; in pipelines it streams plain output. Common pattern is `fit-editor batch '*.fit' -- fit-editor validate` for bulk integrity checks.

Note the `--` separator: everything after it is the command to run, not flags for `batch` itself.

## Output-handling tips for agents

- **Always quote glob patterns** for `batch` so the shell doesn't expand them prematurely.
- **Use `--no-color`** whenever you intend to parse stdout — it strips ANSI escapes and keeps the format stable.
- **Prefer `export -f json` over `dump`** for any structured extraction. `dump` is for humans.
- **Tables in `info` / `summary` use box-drawing characters** when colored, ASCII when `--no-color`. Don't regex against specific characters; if you need structured info, export instead.
- **Exit codes:** `0` = success, `1` = generic failure (decode error, validation failed, batch had failures), other = IO. `batch` returns 1 if any sub-invocation failed.
- **Decode warnings** (`-v`) go to stderr, not stdout — they won't pollute captured output unless you redirect 2>&1.

## Gotchas

- **`encode` requires a JSON shape that matches `export`.** Hand-written JSON will fail unless the top-level `{"messages": [...]}` structure and per-message `{type, index, fields}` shape are preserved. When in doubt, export first as a template.
- **`--set` only modifies existing fields.** It does not add new fields or new messages. For that, edit the JSON and re-encode.
- **GPX export silently drops files with no GPS data.** If `track.gpx` looks empty, the activity had no `position_lat`/`position_long` — probably an indoor workout.
- **Timestamps in FIT use a Garmin epoch (1989-12-31 UTC).** `fit-editor` converts them to RFC3339 in JSON/dump output, so you generally don't have to think about it — but if you see suspiciously old dates (1989-1990) in raw output, that's a 0/missing timestamp.
- **Path separator on `--set`:** `session.total_distance` modifies the **first** session message. Use `session[0]`, `session[1]`, … to disambiguate when there are multiple.

## Discovering more

- `fit-editor <subcommand> --help` is authoritative for flag details.
- For shell setup, `fit-editor completion <bash|zsh|fish|powershell|elvish>` emits a completion script.
- For deep FIT format questions (definition messages, base types, scale/offset semantics), see `docs/FIT_FORMAT.md` in the upstream repo — but for normal user tasks, the CLI surface above is enough.
