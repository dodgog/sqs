# sqs usage

## Setup

```sh
# Initialize in current directory
sqs init

# Or point at an existing folder
sqs --root ~/my-stuff tui
```

`sqs init` creates:
- `sqs.toml` — project config
- `tasks/` — item storage (flat folder of .md files)
- `tasks/lists.yaml` — list definitions
- `.gitignore` entry for `cache/`

## CLI

### Add items

```sh
sqs add "Buy groceries"                  # add to default list (inbox)
sqs add --id myid "Custom ID item"       # explicit ID
sqs add --content "some body" "Title"    # with body, skip editor
```

### List and search

```sh
sqs list              # dashboard (now + next + inbox)
sqs list now          # items in a specific list
sqs find "groceries"  # search title + body
```

### Show, edit, move, delete

```sh
sqs show abc1         # show item detail (prefix match works)
sqs edit abc1         # open in $EDITOR
sqs move abc1 done    # move to another list
sqs delete abc1       # delete permanently
```

### Config and diagnostics

```sh
sqs config            # show resolved configuration
sqs doctor            # check storage health
```

## TUI

```sh
sqs tui
```

### Panes

- **Left** — list of sublists (now, next, later, inbox, done, all)
- **Center** — items in the selected list
- **Right** — preview of the selected item

Switch panes with `h`/`l` or `space`.

### Navigation

- `j`/`k` — move cursor up/down within a list
- `<`/`>` — switch to previous/next list
- `[`/`]` — jump to first/last item
- `g`/`G` — same as `[`/`]`
- `Tab` — cycle lists

### Reordering

- `J`/`K` — move the current item up/down in the list
- At the boundary of a list, J/K crosses into the adjacent list
- `{`/`}` — move item to top/bottom of its list

### Visual selection

Press `v` or `V` to enter visual mode. Extend the selection with `j`/`k`.

- `J`/`K` — move the selected block as a unit
- If the selection spans multiple lists, J/K consolidates first (moves items into one list), then moves the block
- `<`/`>` — send the entire selection to an adjacent list
- `{`/`}` — move selection to top/bottom
- `m` — pick a target list from a menu
- `Esc` — cancel selection

### Adding items

- `o` or `a` — add item **after** cursor
- `O` or `i` — add item **before** cursor
- Tab/Shift-Tab to change target list in the add form
- Enter to create, Esc to cancel

In the "all" view, items are added to the same list as the cursor.

### Other actions

- `e` — edit in `$EDITOR` (or `$VISUAL`)
- `x` — delete (with confirmation)
- `m` — move to a specific list (press first letter or number to pick)
- `/` — search across all items
- `r` — refresh from disk

### Sidebar

- `j`/`k` — navigate lists
- `J`/`K` — reorder lists (saved to `lists.yaml`)
- `v` + `j`/`k` + `J`/`K` — select and reorder multiple lists

### All view

The bottom entry in the sidebar. Shows all items grouped by list with headings. Empty lists are displayed. The order follows the sidebar arrangement. Reordering items across headings moves them between lists.

## File format

Items are `.md` files with YAML frontmatter:

```yaml
---
title: Buy groceries
list: inbox
order: 2.0
created_at: 2026-04-20T10:00:00Z
updated_at: 2026-04-20T10:00:00Z
---

- Milk
- Eggs
- Bread
```

- Filename is the ID (e.g. `x3km.md`)
- `list` — which list the item belongs to
- `order` — display position within the list (lower = higher)
- Body is freeform markdown

## Config discovery

sqs looks for `sqs.toml` starting from the current directory and walking up (like `Cargo.toml`). If not found, falls back to `$SQS_ROOT` env var or `~/.config/sqs/config.toml`.

## Adapters

sqs uses a pluggable adapter system. The built-in adapter (`markdown-todolists`) stores items as markdown files. The adapter trait defines operations for scanning, creating, moving, reordering, and deleting items — any backend can implement it.
