# Contributing

## Workspace Layout

This repository uses a Cargo workspace with these crates:

- `crates/core`: shared domain model and cross-crate types.
- `crates/frontend`: repository scanning and source parsing.
- `crates/engine`: analysis and extraction passes.
- `crates/report`: output shaping and serialization.
- `crates/cli`: command-line entrypoint and orchestration.

Use Oxc types for source metadata when available. In particular, span data should come from `oxc_span::Span` through `core` so all crates share one canonical representation.
The parser pipeline should be driven by Oxc crates (`oxc_parser`, `oxc_allocator`, `oxc_span`) from `frontend`.

Keep business logic in libraries and keep the CLI thin.

## Folder Usage

- Place reusable domain structs in `crates/core/src`.
- Place parser and file-discovery code in `crates/frontend/src`.
- Place extraction and analysis passes in `crates/engine/src`.
- Place output formatters in `crates/report/src`.
- Place argument parsing and pipeline wiring in `crates/cli/src/main.rs`.

## Test Philosophy

Prefer tests that assert meaningful invariants and behavior over brittle full-file snapshots.

- Unit tests should prove one specific semantic behavior.
- Integration tests should run the pipeline against realistic fixture folders.
- Structured JSON assertions are preferred over full JSON snapshot matching unless a full snapshot is the highest-signal option.

## Fixture Conventions

Use fixture mini-projects under crate test directories. Planned convention:

- `crates/engine/tests/fixtures/<case>/input/...`
- Optional `crates/engine/tests/fixtures/<case>/notes.md` for scenario intent.

Each fixture should target a single behavior or edge case and include only files needed for that case.

## Development Commands

Run these before opening a PR:

- `cargo fmt --all --check`
- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace`

Run the CLI locally:

- `cargo run -p cli -- --help`

## Rust Style Expectations

Follow standard Rust conventions and idioms. Prioritize human readability:

- use meaningful names for variables, functions, and types
- avoid cryptic abbreviations and one-letter names except in very local, obvious loops
- keep module boundaries clear and responsibilities focused
