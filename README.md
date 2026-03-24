# mnf

![Rust](https://img.shields.io/badge/Rust-2024-orange?logo=rust)
![License](https://img.shields.io/badge/License-MIT-green.svg)
![Interface](https://img.shields.io/badge/Interface-CLI%20%2B%20TUI-blue)

`mnf` is a Rust terminal app for finding likely-available Minecraft usernames.

It supports both a styled CLI mode and an interactive TUI mode, lets you search by target length and optional prefix, and can export found names to text or CSV.

Important: this project uses public Mojang profile lookups, so results are shown as `likely available`, not guaranteed available.

## Highlights

- Search usernames with a length from `3` to `10`
- Filter by starting character or full prefix such as `e`, `ab`, or `mc_`
- Use either:
  - a polished one-shot CLI
  - an interactive TUI dashboard
- Traverse the candidate space in random order without repeating candidates
- Show search progress, hit count, batch count, and stop reason
- Save CLI results with `--save` as plain text or CSV
- Run on a single Rust binary with shared search logic for both interfaces

## Quick Start

### Prerequisites

- Rust toolchain with `cargo`

### Build

```bash
cargo build
```

### Run the CLI

```bash
cargo run -- cli --length 4 --starts-with e --results 3 --max-checks 20
```

This mode prints a styled terminal summary, live progress, highlighted hits, and a final result block.

### Save results to a file

```bash
cargo run -- cli --length 4 --starts-with e --results 10 --max-checks 200 --save names.txt
```

```bash
cargo run -- cli --length 4 --starts-with e --results 10 --max-checks 200 --save names.csv
```

- Non-`.csv` paths are written as plain text, one name per line
- `.csv` paths are written with a `name` header

### Run the TUI

```bash
cargo run -- tui
```

Default TUI values:

- `length = 4`
- `prefix = ""`
- `results = 10`
- `max_checks = 200`

## CLI Options

`mnf` exposes two subcommands:

- `cli` - run a single search from the command line
- `tui` - launch the interactive terminal UI

CLI flags:

- `--length <u8>` - target username length
- `--starts-with <text>` - optional prefix
- `--results <usize>` - target number of likely-available names to collect
- `--max-checks <usize>` - hard cap on how many candidates to check
- `--save <path>` - optional output file path

Example:

```bash
cargo run -- cli --length 5 --starts-with mc --results 20 --max-checks 300 --save mc-names.csv
```

## Examples

### Find 4-character names starting with `e`

```bash
cargo run -- cli --length 4 --starts-with e --results 10 --max-checks 200
```

### Search random 6-character names with no prefix

```bash
cargo run -- cli --length 6 --results 15 --max-checks 400
```

### Save a larger batch to CSV

```bash
cargo run -- cli --length 5 --starts-with mc --results 50 --max-checks 1000 --save mc-names.csv
```

### Save plain text results

```bash
cargo run -- cli --length 4 --starts-with a --results 25 --max-checks 500 --save names.txt
```

### Open the TUI with custom startup values

```bash
cargo run -- tui --length 5 --starts-with mc --results 25 --max-checks 400
```

## TUI Controls

Inside the TUI:

- `Enter` starts or stops a search
- `Tab` and arrow keys move between fields
- typing edits the selected field when idle
- editing is locked while a search is running
- `q` or `Esc` quits

## Search Behavior

- Supported target lengths are `3..=10`
- Allowed characters are `A-Z`, `a-z`, `0-9`, and `_`
- The prefix cannot be longer than the target length
- Candidate generation is randomized and non-repeating within a search run
- Search results come from public Mojang profile lookups
- The tool reports `likely available` because public lookup is not a guaranteed registration check

## Project Structure

```text
.
├── Cargo.toml
├── LICENSE
├── README.md
├── README.ko.md
├── docs/
│   └── superpowers/
│       └── plans/
│           └── 2026-03-24-minecraft-name-finder.md
└── src/
    ├── checker.rs
    ├── cli.rs
    ├── generator.rs
    ├── lib.rs
    ├── main.rs
    ├── model.rs
    ├── output.rs
    ├── search.rs
    ├── tui/
    │   └── mod.rs
    └── validation.rs
```

## Module Overview

- `src/main.rs` - entry point and subcommand dispatch
- `src/cli.rs` - styled CLI experience and `--save` integration
- `src/tui/mod.rs` - interactive terminal dashboard
- `src/search.rs` - shared search loop, progress events, and stop conditions
- `src/checker.rs` - Mojang lookup client with retry/fallback behavior
- `src/generator.rs` - randomized, non-repeating candidate generation
- `src/validation.rs` - validation for length, prefix, and option bounds
- `src/model.rs` - shared search models, summaries, and events
- `src/output.rs` - file export helpers for text and CSV output

## Technology Stack

- Rust 2024 edition
- `tokio` for async execution
- `reqwest` + `serde` for Mojang API requests and parsing
- `clap` for CLI argument parsing
- `ratatui` + `crossterm` for the terminal UI
- `indicatif` + `owo-colors` for styled CLI output
- `rand` for randomized candidate traversal
- `anyhow` for error handling

## Development

Common local checks:

```bash
cargo fmt
cargo test
cargo check
```


## Testing

The current test suite covers:

- input validation
- randomized candidate generation
- result export formatting
- Mojang response classification
- shared search stop and cancellation behavior

Run all tests with:

```bash
cargo test
```

## Contributing

If you extend the project:

- keep the `likely available` wording unless the lookup source becomes authoritative
- keep CLI and TUI on top of the same shared search engine
- keep file responsibilities focused instead of mixing UI, network, and validation concerns
- run `cargo fmt`, `cargo test`, and `cargo check` before handing off changes

## License

This project is licensed under the MIT License. See `LICENSE`.
