# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog and this project uses SemVer tags (`vX.Y.Z`).

## [Unreleased]

### Changed

- `release.toml` auto-promotes the `[Unreleased]` CHANGELOG block on each release; cargo-dist uses the matching `[X.Y.Z]` section as the GitHub Release body.
- `dist-workspace.toml` and `release.yml` pinned to cargo-dist `0.30.2` to match the version available in the local nix-darwin flake.
- `RELEASING.md` rewritten: prerequisites now point at the system flake's `pkgs.rustup` / `pkgs.cargo-release` / `pkgs.cargo-dist` (no per-project dev shell).

### Removed

- Obsidian vault and daily-notes coupling: schema fields, doctor checks, task frontmatter field, related tests. Anyone with `obsidian_vault_dir` or `daily_notes_dir` in their `config.toml` will get a parse error and must remove those keys.
- Legacy `Task`/`Queue`/`TaskRepo`/`QueueDirs`/`TaskFrontmatter` types, `domain::filter`, `domain::id`, `storage::repo`, `storage::format`, `storage::id_state`, `app::operations` — all dead code from the `tqs` fork. The live CLI/TUI exclusively use `Adapter` → `Item`/`ListDef`.
- `sqs doctor` no longer scans task files or id-generator state files; the `--fix` flag is now a no-op.
- Stale `ARCHITECTURE.md`.

## [0.3.3] - 2026-04-20

### Added

- Folder-based storage for sublists in the markdown adapter.

## [0.3.2] - 2026-04-20 - sqs rewrite

### Added

- **Adapter abstraction** — pluggable `Adapter` trait (`Item`, `ListDef`) decouples the CLI/TUI from any specific storage backend. First adapter: `markdown-todolists`.
- **SQLite cache module** — per-adapter cache for fast metadata lookups (`src/cache/mod.rs`). Schema stores items and list definitions; body is read on-demand from the adapter.
- **New frontmatter format** — items use `list:` and `order:` fields. Flat-folder storage with `lists.yaml` sidecar for list definitions.
- **New identity scheme** — random `[a-z]{4}[0-9]{4}` IDs (e.g., `abcd1234`) replace the sequential Crockford base-32 allocator.
- **`sqs init` command** — scaffolds `sqs.toml`, `tasks/` directory, `lists.yaml`, and `cache/` with `.gitignore` entry.
- **Vim-style TUI keys** — `g`/`G` for top/bottom, `v` for visual mode with selection extension via `j`/`k`, bulk move via `v` + `m`.
- **Seed data** — `examples/seed/` with sample items across 5 lists and a `lists.yaml`.

### Changed

- **Renamed `tqs` to `sqs`** — binary, crate, config paths (`~/.config/sqs/`), environment variable (`SQS_ROOT`), state directory (`.sqs`).
- **"Queue" terminology replaced with "list"** throughout CLI output and error messages.
- **Simplified `operations` module** — `mark_done` no longer integrates with daily notes.

### Removed

- **CLI commands**: `now`, `inbox`, `start`, `done`, `triage` — use `sqs list <name>` and `sqs move <id> <list>` instead.
- **Daily notes integration** — Obsidian daily note append-on-completion feature.
- **Fuzzy task picker** — interactive picker removed; task resolution uses exact/prefix/title matching.
- **Dependencies**: `fuzzy-matcher`, `libc` removed.

## [0.3.1] - 2026-04-09

### Fixed

- TUI now redraws correctly when the terminal window is resized, fixing broken layout and leftover characters on screen.

## [0.3.0]

### Added

- **Interactive TUI dashboard** — running `tqs` with no arguments on a TTY now launches a full-screen terminal UI powered by ratatui + crossterm.
  - Three-panel layout: sidebar (queues + counts), task list, and task detail pane.
  - Focus-based panel navigation: `h/l` or arrow keys move focus between panels; `j/k` navigates queues, tasks, or scrolls detail depending on the focused panel.
  - `Tab` and `1-6` cycle or jump to queues from any panel.
  - Inline task creation with `a` (title + queue selector via Tab/Shift-Tab).
  - Edit tasks in `$EDITOR` with `e` (suspends and restores the TUI).
  - Task actions: `d` done, `s` start, `m` move (then pick queue), `x` delete (with confirmation), `r` refresh.
  - Dedicated triage mode with `t` — cycles through inbox tasks one at a time using the same keybindings as normal mode (plus `Space` to skip). Shows a summary on completion.
  - Search mode with `/` — filters tasks across all queues in real-time.
  - Sidebar groups queues into sections (active / triage / archive) with visual separators.
  - "All" virtual view shows every task across all queues.
- `--no-tui` flag to disable the interactive dashboard and show the plain text output instead.

### Changed

- Extracted shared `mark_done` logic into `app::operations` (used by CLI `done`, CLI `triage`, and TUI).
- Running `tqs` with a valid config but no tasks now opens the dashboard (where you can press `a` to add) instead of showing the getting-started guide.

## [0.2.3] - 2026-03-31

### Added

- `delete` command — permanently removes a task file. Supports `--interactive` (`-i`) flag to prompt for confirmation before deleting.

### Changed

- Dashboard (`list` without a queue) now shows **now**, **next**, a separator, then **inbox** (previously: now, inbox, next).

## [0.2.2] - 2026-03-24

### Added

- `triage` command — interactively walk through inbox tasks and dispatch them to queues, mark done, edit, or delete.
- `doctor` now detects orphaned ID-generator state files in `.tqs/id-generator/` and warns about them.
- `doctor --fix` removes orphaned state files automatically.

### Fixed

- Config file paths starting with `~` (e.g. `tasks_root = "~/o/tasks"`) are now correctly expanded to the user's home directory instead of being treated as relative to the config file location.

## [0.2.1] - 2026-03-23

### Added

- `start` command — moves a task to the now queue (shortcut for `move <task> now`).
- Running `tqs` with no arguments now shows the task dashboard if a config and tasks exist, or a getting-started guide otherwise.

## [0.2.0] - 2026-03-18

Complete rework, changed totally the design - it went from a "per project task queue" to a "personal todo list with some optional Obsidian integration". I'm not going to document the changes because it makes little sense; go read the README or ARCHITECTURE if you are interested in the new system.

## [0.1.2] - 2026-02-25

### Fixed

- Fuzzy subcommand expansion now works when global options with values appear before the command shorthand, so `tqs --root /tmp l` resolves to `list`.

## [0.1.1] - 2026-02-24

### Added

- Homebrew formula.

## [0.1.0] - 2026-02-24

### Added

- GitHub Actions release automation with `cargo-dist` and tag-driven publishing.
- Initial release.
