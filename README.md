# sqs

A vim-style terminal tool for reordering items across named lists. Items are Markdown files with YAML frontmatter. Lists are defined in a `lists.yaml` sidecar. Everything lives in a flat folder.

## Install

### From source

```sh
cargo install --path .
```

### Nix

```sh
nix build github:dodgog/sqs
# or run directly
nix run github:dodgog/sqs -- tui
```

### Local build

```sh
cargo build --release
cp target/release/sqs ~/.local/bin/sq
```

## Quick start

```sh
sqs init        # creates sqs.toml, tasks/, lists.yaml
sqs tui         # launch the interactive TUI
sqs add "thing" # add from CLI
sqs list        # print items
```

## TUI

Three panes: lists (left), items (center), preview (right).

### Normal mode

| Key | Action |
|-----|--------|
| `j` / `k` | navigate items (stops at list boundary) |
| `J` / `K` | reorder item within list; at boundary, crosses into adjacent list |
| `<` / `>` | switch to prev/next list |
| `[` / `]` | jump to first/last item in current list |
| `{` / `}` | move item to top/bottom of current list |
| `h` / `l` | switch pane |
| `space` | toggle sidebar / items pane |
| `v` / `V` | enter visual selection mode |
| `o` / `a` | add item after cursor |
| `O` / `i` | add item before cursor |
| `m` | move item to list (picker) |
| `e` | edit item in $EDITOR |
| `x` | delete item |
| `/` | search |
| `r` | refresh |
| `q` / `Esc` | quit |

### Visual mode

Select multiple items with `j`/`k`, then act on the selection:

| Key | Action |
|-----|--------|
| `j` / `k` | extend selection |
| `J` / `K` | reorder selection as a block; consolidates across lists first |
| `<` / `>` | send selection to prev/next list |
| `{` / `}` | move selection to top/bottom of list |
| `[` / `]` | extend selection to first/last item |
| `m` | move selection to list (picker) |
| `h` / `l` | exit visual, switch pane |
| `Esc` / `v` | cancel selection |

### Sidebar

| Key | Action |
|-----|--------|
| `j` / `k` | navigate lists |
| `J` / `K` | reorder lists (persisted to lists.yaml) |
| `v` + `j`/`k` + `J`/`K` | visual select and reorder multiple lists |

### All view

The last entry in the sidebar. Shows every item grouped by list with headings. Empty lists are shown. Reordering across list boundaries moves items between lists. The display order follows the sidebar list order.

## File format

Each item is a `.md` file in the tasks folder. Filename is the ID (4 alphanumeric chars).

```yaml
---
title: My item
list: now
order: 0.0
created_at: 2026-04-19T12:00:00Z
updated_at: 2026-04-19T12:00:00Z
---

Freeform body text.
```

`lists.yaml` defines the available lists and their display order:

```yaml
- name: now
  display: now
  order: 0.0
- name: next
  display: next
  order: 1.0
- name: later
  display: later
  order: 2.0
- name: inbox
  display: inbox
  order: 3.0
- name: done
  display: done
  order: 4.0
```

## CLI

```
sqs init              scaffold sqs.toml + tasks/ + lists.yaml
sqs tui               launch TUI
sqs add [--id ID] T   add item with title T
sqs list [LIST]        print items
sqs show ID            print item detail
sqs move ID LIST       move item to list
sqs edit ID            edit in $EDITOR
sqs delete ID          delete item
sqs find QUERY         search items
sqs config             show config
sqs doctor             check health
```

## Config

`sqs.toml` is discovered by walking up from the current directory (like `Cargo.toml`). Falls back to `~/.config/sqs/config.toml` or `$SQS_ROOT`.

```toml
default_adapter = "markdown-todolists"

[adapters.markdown-todolists]
root = "./tasks"
```

## Architecture

Pluggable adapter layer. The TUI and CLI work through the `Adapter` trait. The shipped adapter (`markdown-todolists`) stores items as flat `.md` files. Other adapters can implement the same trait for different backends.
