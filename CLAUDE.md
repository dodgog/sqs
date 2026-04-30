# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is sqs

A vim-style terminal tool for reordering items across named lists. Items are Markdown files with YAML frontmatter. Lists are defined in a `lists.yaml` sidecar. Everything lives in a flat folder.

## Project Overview

**Type**: Rust CLI with interactive TUI (crossterm + ratatui)  
**Architecture**: Pluggable adapter pattern for storage backends  
**Primary Interface**: Interactive terminal UI with fallback CLI commands  
**Version**: 0.3.1 (2024 edition)

---

## Core Architectural Patterns

### 1. **Adapter Trait Pattern** (`src/adapter/mod.rs`)

The entire codebase revolves around the `Adapter` trait, which abstracts storage:

```rust
pub trait Adapter: Send {
    fn lists(&self) -> Vec<ListDef>;                    // Get list definitions
    fn set_lists(&mut self, lists: &[ListDef]) -> ..;   // Persist list order
    fn scan(&self) -> Result<Vec<Item>, AppError>;      // Load all items
    fn find_item(&self, ext_id: &str) -> ...;           // Lookup one item
    fn create_item(...) -> Result<(Item, PathBuf), ..>; // Create + return path
    fn move_item(&mut self, ext_id, target_list) -> ..; // Change item's list
    fn reorder_items(&mut self, list, ids) -> ...;      // Persist order
    fn delete_item(&mut self, ext_id) -> ...;           // Remove item
    fn editor_path(&self, ext_id) -> ...;               // Get file for $EDITOR
    fn apply_edit(&mut self, ext_id, path, content) -> Result<EditOutcome, ..>; // Validate + persist
    fn finalize_add_edit(...) -> Result<(), ..>;        // After user edit
}
```

**Shipped Implementation**: `MarkdownTodolistsAdapter` (`src/adapters/markdown_todolists/mod.rs`)
- Stores items as flat `.md` files with YAML frontmatter
- List definitions in `lists.yaml`
- Uses FNV hash for change detection (`content_hash` field)

**Why This Matters**: Both the TUI and CLI work exclusively through the adapter. New backends only need to implement this trait.

---

### 2. **Domain Model** (`src/adapter/mod.rs`)

**Item**: Core task representation
- `ext_id`: 4-char alphanumeric identifier (filename without .md)
- `title`: Display name
- `body`: Freeform content (after YAML frontmatter)
- `list`: Which list it belongs to
- `order`: Float for positioning (allows insertion without reordering all)
- `content_hash`: u64 for change detection

**ListDef**: Metadata about a list
- `name`: Internal identifier (e.g., "now")
- `display`: Human-readable label
- `order`: Float for list ordering

**EditOutcome**: Result of applying an editor change
- `Unchanged`: User made no changes
- `Applied`: Changes were persisted

---

## Module Structure

```
src/
├── lib.rs                    # Re-exports all modules
├── adapter/                  # Core trait definition
├── adapters/
│   └── markdown_todolists/   # Shipped storage backend
│       ├── frontmatter.rs    # YAML parsing/serialization
│       ├── identity.rs       # ID generation
│       └── io.rs             # File I/O helpers
├── app/
│   ├── service.rs            # Entry point: parse CLI, dispatch to TUI or CLI handlers
│   └── app_error.rs          # Error type (thiserror-based)
├── cache/                    # SQLite cache (in-memory or file-based)
│   └── mod.rs                # SqliteCache: upsert/query/reconcile
├── cli/
│   ├── mod.rs                # CLI setup
│   ├── args.rs               # clap-based argument parsing
│   ├── handlers.rs           # Main dispatcher (calls command handlers)
│   ├── fuzzy.rs              # Expands abbreviated commands
│   └── commands/             # Individual command implementations
│       ├── add.rs, delete.rs, move_cmd.rs, etc.
│       └── helpers.rs        # Shared CLI utilities
├── io/
│   ├── input.rs              # User input (prompts, dialogs)
│   └── output.rs             # Pretty-printing
├── storage/
│   ├── config.rs             # Config discovery + loading (sqs.toml → ResolvedConfig)
│   ├── doctor.rs             # Configuration diagnostics
│   └── editor.rs             # $EDITOR integration
├── tui/                      # Interactive terminal UI
│   ├── mod.rs                # Entry point: setup terminal, run event loop
│   ├── app_state.rs          # TuiApp (main state) + navigation enums
│   ├── actions.rs            # Action functions (move_to_list, etc.)
│   ├── event.rs              # Event polling + key handling
│   ├── ui.rs                 # Ratatui rendering pipeline
│   └── widgets/              # Widget components
│       ├── sidebar.rs        # List picker
│       ├── task_list.rs      # Item list
│       ├── detail.rs         # Item preview
│       ├── add_form.rs       # Quick-add form
│       ├── status_bar.rs     # Bottom status line
│       └── mod.rs
└── test_support.rs           # Test fixtures
```

---

## Data Flow

### TUI (Interactive Mode)

1. **Startup** (`src/tui/mod.rs::run()`)
   - Initialize terminal (raw mode, alternate screen)
   - Create `TuiApp` from adapter
   - Enter event loop

2. **Event Loop** (`src/tui/mod.rs::run_loop()`)
   - Poll crossterm events (250ms timeout)
   - Route key events through `event::handle_key()`
   - Execute actions (modify `app.items`, call adapter methods)
   - Handle side effects (editor suspend, quit)
   - Render with `ui::draw()` if `needs_redraw`

3. **Navigation State** (`src/tui/app_state.rs::TuiApp`)
   - `items`: Cache of all items (from adapter.scan())
   - `active_sidebar_index`: Which list is selected
   - `task_list_state`: Which item in the current list is selected
   - `mode`: Normal | Visual | AddForm | ConfirmDelete | MoveTarget | Search
   - `focused_panel`: Sidebar | TaskList | Detail
   - `status_message`: Transient feedback (3s TTL)

4. **Mode Handling**
   - **Normal**: Vim-like navigation (hjkl, j/k, J/K for reorder)
   - **Visual**: Multi-select with `v`, extend with `j/k`, operate on selection
   - **AddForm**: Quick-add with title + list picker
   - **ConfirmDelete**: Y/N confirmation
   - **MoveTarget**: List picker for moving item(s)
   - **Search**: Fuzzy search results with navigation

5. **Rendering** (`src/tui/ui.rs`)
   - Three panes: Sidebar (left), TaskList (center), Detail (right)
   - List counts updated from `app.list_counts()`
   - Current items filtered via `app.current_items()` (handles "All" view)

### CLI (Command Mode)

1. **Entry** (`src/app/service.rs::run()`)
   - Parse args with clap
   - Expand fuzzy aliases (e.g., `a` → `add`)
   - Dispatch to `handlers::handle()`

2. **Routing** (`src/cli/handlers.rs`)
   - Route to command handler (e.g., `add::handle_add()`)
   - Each handler resolves config, initializes adapter, performs action
   - Return AppError on failure

3. **Error Handling**
   - All errors from handlers convert to eprintln! + exit code
   - Exit codes: 0 (ok), 1 (runtime error), 2 (usage error)

---

## Key Types & Abstractions

### AppError (thiserror)

```rust
pub enum AppError {
    Message(String),                    // Generic error
    Usage(String),                      // CLI usage mistake
    NotFound { id: String },            // Item not found
    AmbiguousTaskRef { query: String }, // Multiple matches
    NoTty,                              // Needs terminal
    InvalidTaskFile { path, reason },   // Bad YAML/format
    PathTraversalAttempt(String),       // Security
    // ... std::io, serde_yaml, FormatError, dialoguer
}
```

Maps to exit codes: Usage → 2, others → 1.

### Mode (TUI)

Controls what keys do:
- **Normal**: Standard navigation + commands
- **Visual { anchor }**: Multi-select from `anchor` to cursor
- **AddForm { title, list, insert_at }**: Create new item
- **ConfirmDelete { task_id }**: Confirm before delete
- **MoveTarget**: Pick destination list
- **Search { query, results, list_state }**: Jump to item

### ListFilter

Controls which items are shown:
- `Single(name)`: Just this list
- `All`: All items, grouped by list in sidebar order

---

## Config Resolution (`src/storage/config.rs`)

**Discovery**: Walks up from current directory looking for `config.toml` (like cargo)  
**Fallbacks**: `~/.config/sqs/config.toml`, `$SQS_ROOT` env var

**ResolvedConfig**:
```toml
default_adapter = "markdown-todolists"

[adapters.markdown-todolists]
root = "./tasks"
```

Returns `ResolvedConfig` with:
- `tasks_root`: Where `.md` files live
- `state_dir`: For internal state (cache, etc.)

---

## Caching Layer (`src/cache/mod.rs`)

**SqliteCache** in-memory or file-based:

```sql
CREATE TABLE lists (
    name TEXT PRIMARY KEY,
    display TEXT NOT NULL,
    order_key REAL NOT NULL
);

CREATE TABLE items (
    ext_id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    list TEXT NOT NULL,
    order_key REAL NOT NULL,
    content_hash INTEGER NOT NULL
);
```

**Methods**:
- `upsert_lists()`, `upsert_items()`: Insert or update
- `query_lists()`, `query_items(list?)`: Fetch from cache
- `reconcile()`: Update cache from adapter + remove stale entries
- `remove_stale()`: Clean up deleted items

**Note**: `body` is NOT cached; only metadata is. Cache is for fast startup and search.

---

## Dependencies

- **Terminal**: `crossterm` (event polling), `ratatui` (rendering)
- **Config**: `toml`, `serde`, `serde_yaml`
- **Error Handling**: `thiserror`
- **CLI**: `clap` (parser), `dialoguer` (prompts)
- **Time**: `chrono` (timestamps)
- **ID Gen**: `rand`
- **Shell**: `shell-words` (parse $EDITOR)
- **DB**: `rusqlite` (bundled sqlite)
- **Testing**: `assert_cmd`, `assert_fs`, `tempfile`, `predicates`

---

## Common Workflows

### Adding an Item (TUI)

1. Press `o` (after cursor) or `O` (before)
2. Enter title → press Enter
3. Modal allows picking list (arrows + Enter)
4. Calls `adapter.create_item()` with `order` = new offset from cursor
5. If user pressed `a`/`O` instead, spawns $EDITOR for body
6. Calls `adapter.finalize_add_edit()` to persist changes

### Reordering Items

1. **Within list**: Press `J` (down, bumps others up) or `K` (up)
   - Reorder: Calculates new `order` values, calls `adapter.reorder_items()`
2. **Across lists**: In visual mode, `<`/`>` move selection to adjacent list
   - Calls `adapter.move_item()` on each, then reorder

### Searching

1. Press `/`
2. Type query (substring search)
3. Navigate with `j`/`k`, enter to jump to result
4. Calls fuzzy matching on titles + bodies

### Editing

1. Press `e` on item
2. Launches `$EDITOR $file`
3. TUI suspends, waits for editor exit
4. Calls `adapter.apply_edit()` to validate + persist
5. Refresh display

---

## Notable Implementation Details

### Order Field

Items use `f64` for ordering, not a sequence number. This allows insertion without shifting all subsequent items:
- Insert at position 0.5 (between 0.0 and 1.0)
- Avoids cascading updates on reorder

### Content Hash

Used to detect changes after $EDITOR:
- Computed when item created/loaded
- If unchanged after edit, `adapter.apply_edit()` returns `EditOutcome::Unchanged`
- Prevents unnecessary writes

### Visual Selection

Stored as `anchor` + cursor position:
- `anchor`: Start of selection
- Selection = min..=max of [anchor, cursor]
- `j`/`k` extend by moving cursor
- `J`/`K` reorder selection as block

### All View

Last sidebar entry. When selected:
- `current_items()` returns all items sorted by:
  1. Sidebar list order (list_order vec)
  2. Within-list order field
- Reordering across list boundaries moves items between lists

### Status Messages

Appear at bottom for 3 seconds:
- Set with `app.set_status(msg)`
- Expire via `Instant` check in render loop
- `needs_redraw` set when message added/expires

---

## Key Functions to Know

### Adapter Protocol

| Function | When Called | Side Effects |
|----------|------------|--------------|
| `lists()` | Startup, after list reorder | None (read) |
| `scan()` | Startup, refresh (r), item operations | Rebuilds `app.items` |
| `create_item()` | Add form submit | Creates file |
| `move_item()` | Move or list switch in visual | Rewrites item |
| `reorder_items()` | Drag/J/K | Rewrites multiple items |
| `delete_item()` | x or confirm delete | Deletes file |
| `apply_edit()` | After $EDITOR closes | Rewrites item |

### App State

| Method | Returns | Notes |
|--------|---------|-------|
| `current_items()` | `Vec<&Item>` | Filtered + sorted |
| `selected_item()` | `Option<&Item>` | None if empty |
| `list_counts()` | `ListCounts` | For sidebar display |
| `active_filter()` | `ListFilter` | Single or All |
| `refresh()` | Result | Rescans adapter |
| `jump_to_list()` | None | Navigate to list |
| `select_*_task()` | None | Move cursor |

### Event Handling

`event::handle_key()` routes through `Mode`:
- Normal: `handle_normal_key()` — most vim keys
- Visual: `handle_visual_key()` — selection ops
- AddForm: `handle_add_form_key()` — input + list picker
- ConfirmDelete: `handle_confirm_delete_key()` — y/n
- MoveTarget: `handle_move_target_key()` — list picker
- Search: `handle_search_key()` — navigate results

Returns `SideEffect`:
- `None`: State changed, redraw
- `Quit`: Exit TUI
- `SuspendForEditor { task_id }`: Run $EDITOR

---

## Testing

- **Unit tests**: embedded in modules (`#[cfg(test)]` blocks)
- **Integration tests**: `tests/cli_smoke.rs` (38 tests) and `tests/regression.rs` (5 tests) using `assert_cmd` + `assert_fs`
- **Test fixtures**: `src/test_support.rs` provides `LockedEnv` for env var isolation
- CLI integration tests use `cargo_bin_cmd!("sqs")` with temp dirs

---

## Future Extension Points

### New Adapter

Implement `Adapter` trait with custom storage (e.g., database, cloud sync). Plug into CLI + TUI without changes.

### New Commands

Add to `cli/commands/`, implement handler function, add variant to `Command` enum in `args.rs`, wire in `handlers::handle()`.

### New TUI Mode

Add variant to `Mode` enum, implement `handle_*_key()`, add UI in `widgets/`.

### Cache Strategy

Replace `SqliteCache` with Redis, Postgres, etc. (interface only needs `upsert`, `query`, `remove_stale`, `reconcile`).

---

## Development Commands

```bash
# All three must pass before any change is considered done
cargo fmt --check
cargo clippy -- -D warnings
cargo test

# Build release binary
cargo build --release

# Run locally
cargo run -- tui
cargo run -- add "test"
cargo run -- list

# Run a single test
cargo test test_name
cargo test cache::    # all cache tests

# Debug with backtrace
RUST_BACKTRACE=1 cargo run -- tui
```

Use Conventional Commits. Keep commits short and focused.

---

## Gotchas & Conventions

1. **Path Traversal**: All item IDs are validated to prevent `../../../etc/passwd`
2. **TTY Requirement**: Some CLI operations (e.g., editor) fail without TTY
3. **Markdown Files**: Item files MUST have YAML frontmatter (`---...---`), body after
4. **List Names**: Case-sensitive; `now` ≠ `Now`
5. **Order Field**: Used as float; updates use fractional values between existing
6. **Status TTL**: 3 seconds; hard-coded in `app_state.rs`
7. **Sidebar All Entry**: Always at end; added automatically if adapter returns empty list

---

## Summary for Next Claude

This is a **clean, modular Rust CLI** built around an **adapter pattern**. The TUI is event-driven (crossterm polling) and state-based (TuiApp). Storage is abstracted via the Adapter trait; the shipped markdown backend stores items as flat `.md` files. Config discovery walks up directories. Everything goes through either the TUI or CLI handlers → adapter → storage.

**Key insight**: Nearly all complexity is in the TUI event loop and state machine. Adapter implementations are straightforward (markdown backend ~500 lines). CLI is simple command dispatch.

**To extend**: Implement Adapter or add TUI modes.
