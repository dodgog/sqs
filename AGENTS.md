# AGENT.md

## Project

`sqs` is a Rust CLI for reordering named lists from the terminal. Items are stored as Markdown files with YAML frontmatter, organized by a pluggable adapter layer.

The first adapter (`markdown-todolists`) stores items as `.md` files with `list:` and `order:` frontmatter fields. List definitions live in a `lists.yaml` sidecar. A SQLite cache provides fast metadata lookups.

## Development Commands

- `cargo fmt --check` - formatting
- `cargo clippy -- -D warnings` - lint
- `cargo test` - tests
- `cargo run -- <command>` - run the CLI locally

## Architecture

```
src/
  adapter/mod.rs          Adapter trait, Item, ListDef, EditOutcome
  adapters/
    markdown_todolists/   First adapter: md files + YAML frontmatter
      mod.rs              impl Adapter wrapping TaskRepo
      frontmatter.rs      ItemFrontmatter parsing/rendering
      io.rs               Flat-directory scanner, lists.yaml read/write
      identity.rs         Random [a-z]{4}[0-9]{4} ID generator
  cache/mod.rs            SqliteCache for per-adapter metadata caching
  cli/                    clap-based CLI commands
  tui/                    ratatui TUI with vim-style keys
  app/                    Service entry point, error model, shared operations
  domain/                 Legacy domain types (Task, Queue, ID generation)
  storage/                Legacy storage layer (TaskRepo, config, format)
```

## Rules

- **Always** ensure that `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` are clean before saying "done"
- Use Conventional Commits
- Keep commits short and focused
- Split work into logical commits
- Keep documentation updated when behavior changes
- Ensure CHANGELOG.md gets updated for new features or bug fixes
- When in doubt, _ask_!
