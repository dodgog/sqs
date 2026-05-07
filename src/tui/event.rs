use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;

use crate::app::app_error::AppError;

use super::actions::{self, SideEffect};
use super::app_state::{DeleteScope, FocusedPanel, Mode, SidebarEntry, TagTarget, TuiApp};

pub fn poll_event(timeout: Duration) -> std::io::Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

pub fn handle_key(app: &mut TuiApp, key: KeyEvent) -> Result<SideEffect, AppError> {
    match &app.mode {
        Mode::Normal => handle_normal_key(app, key),
        Mode::Visual { .. } => handle_visual_key(app, key),
        Mode::AddForm { .. } => handle_add_form_key(app, key),
        Mode::AddSublist { .. } => handle_add_sublist_key(app, key),
        Mode::ConfirmDelete { .. } => handle_confirm_delete_key(app, key),
        Mode::CarryToList { .. } => handle_carry_to_list_key(app, key),
        Mode::Search { .. } => handle_search_key(app, key),
        Mode::Find { .. } => handle_find_key(app, key),
        Mode::TagPicker { .. } => handle_tag_picker_key(app, key),
    }
}

/// Enter the tag picker for a set of items or lists. Pre-checks tags
/// already present on every target so unchecking removes them.
fn enter_tag_picker(app: &mut TuiApp, target: TagTarget) {
    let initial = match &target {
        TagTarget::Items(ids) if !ids.is_empty() => app.intersect_item_tags(ids),
        TagTarget::Lists(names) if !names.is_empty() => app.intersect_list_tags(names),
        _ => return,
    };
    app.mode = Mode::TagPicker {
        target,
        cursor: 0,
        selected: initial.clone(),
        new_tag: String::new(),
        initial,
    };
}

/// Enter carry mode with the given item ids. Computes the source list set,
/// switches focus to the sidebar, and sets the standard carry status.
fn enter_carry(app: &mut TuiApp, ids: Vec<String>, prior_anchor: Option<usize>) {
    if ids.is_empty() {
        return;
    }
    let count = ids.len();
    let mut source_lists: Vec<String> = ids
        .iter()
        .filter_map(|id| app.items.iter().find(|it| it.ext_id == *id))
        .map(|it| it.list.clone())
        .collect();
    source_lists.sort();
    source_lists.dedup();
    app.mode = Mode::CarryToList {
        selected_ids: ids,
        source_lists,
        prior_anchor,
        pending_list_delete: Vec::new(),
    };
    app.focused_panel = FocusedPanel::Sidebar;
    app.set_status(format!(
        "Carrying {count} item(s) — L or Enter to drop, Esc to cancel"
    ));
}

fn handle_normal_key(app: &mut TuiApp, key: KeyEvent) -> Result<SideEffect, AppError> {
    match key.code {
        KeyCode::Char('q') => return Ok(SideEffect::Quit),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return Ok(SideEffect::Quit);
        }

        KeyCode::Char('h') | KeyCode::Left => {
            app.focused_panel = app.focused_panel.left();
        }
        KeyCode::Char('l') | KeyCode::Right => {
            app.focused_panel = app.focused_panel.right();
        }
        KeyCode::Char(' ') if app.focused_panel == FocusedPanel::Tags => {
            app.toggle_tag_at_cursor();
        }
        KeyCode::Char(' ') => {
            app.focused_panel = match app.focused_panel {
                FocusedPanel::Sidebar => FocusedPanel::TaskList,
                FocusedPanel::TaskList => FocusedPanel::Sidebar,
                FocusedPanel::Detail => FocusedPanel::Sidebar,
                FocusedPanel::Tags => FocusedPanel::TaskList,
            };
        }

        KeyCode::Char('j') | KeyCode::Down => match app.focused_panel {
            FocusedPanel::Sidebar => app.next_queue(),
            FocusedPanel::TaskList | FocusedPanel::Detail => app.select_next_task(),
            FocusedPanel::Tags => app.select_next_tag(),
        },
        KeyCode::Char('k') | KeyCode::Up => match app.focused_panel {
            FocusedPanel::Sidebar => app.prev_queue(),
            FocusedPanel::TaskList | FocusedPanel::Detail => app.select_prev_task(),
            FocusedPanel::Tags => app.select_prev_tag(),
        },

        KeyCode::Tab => app.next_queue(),
        KeyCode::BackTab => app.prev_queue(),

        KeyCode::Char(c @ '1'..='6') => {
            let index = (c as usize) - ('1' as usize);
            app.select_queue_by_index(index);
        }

        KeyCode::Char('m') => {
            if let Some(item) = app.selected_item() {
                let id = item.ext_id.clone();
                enter_carry(app, vec![id], None);
            }
        }
        KeyCode::Char('x') => {
            if app.focused_panel == FocusedPanel::Sidebar {
                let names = app.selected_sublist_names();
                if !names.is_empty() {
                    app.mode = Mode::ConfirmDelete {
                        scope: DeleteScope::Lists(names),
                    };
                }
            } else if let Some(item) = app.selected_item() {
                app.mode = Mode::ConfirmDelete {
                    scope: DeleteScope::Items(vec![item.ext_id.clone()]),
                };
            }
        }

        KeyCode::Char('/') => {
            app.mode = Mode::Search {
                query: String::new(),
                results: Vec::new(),
                list_state: ListState::default(),
            };
            app.update_search_results();
        }
        KeyCode::Char('f') => {
            app.mode = Mode::Find {
                query: String::new(),
                results: Vec::new(),
                list_state: ListState::default(),
            };
            app.update_find_results();
        }

        // O/i before, o/a after — sublist on sidebar pane, item elsewhere
        KeyCode::Char('O') | KeyCode::Char('i') | KeyCode::Char('o') | KeyCode::Char('a') => {
            let before = matches!(key.code, KeyCode::Char('O') | KeyCode::Char('i'));
            if app.focused_panel == FocusedPanel::Sidebar {
                let insert_at = if before {
                    app.active_sidebar_index
                } else {
                    sidebar_after_index(app)
                };
                app.mode = Mode::AddSublist {
                    name: String::new(),
                    insert_at,
                };
            } else {
                let list = match app.active_filter() {
                    crate::tui::app_state::ListFilter::Single(name) => name,
                    crate::tui::app_state::ListFilter::All => app
                        .selected_item()
                        .map(|i| i.list.clone())
                        .unwrap_or_else(|| crate::adapter::DEFAULT_LIST.to_string()),
                };
                let insert_at = if before {
                    app.task_list_state.selected().unwrap_or(0)
                } else {
                    app.task_list_state.selected().map(|i| i + 1).unwrap_or(0)
                };
                app.mode = Mode::AddForm {
                    title: String::new(),
                    list,
                    insert_at,
                };
            }
        }

        KeyCode::Char('e') => {
            if let Some(item) = app.selected_item() {
                return Ok(SideEffect::SuspendForEditor {
                    task_id: item.ext_id.clone(),
                });
            }
        }

        KeyCode::Char('g') if app.focused_panel == FocusedPanel::TaskList => {
            app.select_first_task_absolute();
        }
        KeyCode::Char('G') if app.focused_panel == FocusedPanel::TaskList => {
            app.select_last_task();
        }

        KeyCode::Char('J') => match app.focused_panel {
            FocusedPanel::Sidebar => {
                if app.tag_filter_active() {
                    app.set_status("List reorder disabled while a tag filter is active");
                } else {
                    app.swap_list_down()?;
                }
            }
            FocusedPanel::TaskList | FocusedPanel::Detail => {
                return actions::reorder_down(app);
            }
            FocusedPanel::Tags => {}
        },
        KeyCode::Char('K') => match app.focused_panel {
            FocusedPanel::Sidebar => {
                if app.tag_filter_active() {
                    app.set_status("List reorder disabled while a tag filter is active");
                } else {
                    app.swap_list_up()?;
                }
            }
            FocusedPanel::TaskList | FocusedPanel::Detail => {
                return actions::reorder_up(app);
            }
            FocusedPanel::Tags => {}
        },

        KeyCode::Char('>') => app.next_queue(),
        KeyCode::Char('<') => app.prev_queue(),

        KeyCode::Char('[') if app.focused_panel != FocusedPanel::Sidebar => {
            app.select_first_task_absolute();
        }
        KeyCode::Char(']') if app.focused_panel != FocusedPanel::Sidebar => {
            app.select_last_task();
        }

        KeyCode::Char('{') if app.focused_panel != FocusedPanel::Sidebar => {
            return actions::move_to_top(app);
        }
        KeyCode::Char('}') if app.focused_panel != FocusedPanel::Sidebar => {
            return actions::move_to_bottom(app);
        }

        KeyCode::Char('=') => return actions::renormalize(app),

        KeyCode::Enter if app.focused_panel == FocusedPanel::Tags => {
            app.toggle_tag_at_cursor();
        }
        KeyCode::Char('t') => match app.focused_panel {
            FocusedPanel::Sidebar => {
                let names = app.selected_sublist_names();
                if !names.is_empty() {
                    enter_tag_picker(app, TagTarget::Lists(names));
                }
            }
            FocusedPanel::TaskList | FocusedPanel::Detail => {
                if let Some(item) = app.selected_item() {
                    enter_tag_picker(app, TagTarget::Items(vec![item.ext_id.clone()]));
                }
            }
            FocusedPanel::Tags => {}
        },

        KeyCode::Char('v') | KeyCode::Char('V')
            if app.focused_panel == FocusedPanel::TaskList
                || app.focused_panel == FocusedPanel::Sidebar =>
        {
            let cursor = if app.focused_panel == FocusedPanel::Sidebar {
                app.active_sidebar_index
            } else {
                app.task_list_state.selected().unwrap_or(0)
            };
            app.mode = Mode::Visual { anchor: cursor };
        }

        KeyCode::Char('r') => {
            app.refresh()?;
            app.set_status("Refreshed");
        }

        _ => {}
    }
    Ok(SideEffect::None)
}

fn sidebar_after_index(app: &TuiApp) -> usize {
    let entries = app.sidebar_entries();
    let idx = app.active_sidebar_index;
    if matches!(entries.get(idx), Some(SidebarEntry::All)) {
        return idx;
    }
    idx + 1
}

fn handle_visual_key(app: &mut TuiApp, key: KeyEvent) -> Result<SideEffect, AppError> {
    let in_sidebar = app.focused_panel == FocusedPanel::Sidebar;
    match key.code {
        KeyCode::Esc | KeyCode::Char('v') => {
            app.mode = Mode::Normal;
        }
        KeyCode::Char('h') | KeyCode::Left => {
            app.mode = Mode::Normal;
            app.focused_panel = app.focused_panel.left();
        }
        KeyCode::Char('l') | KeyCode::Right => {
            app.mode = Mode::Normal;
            app.focused_panel = app.focused_panel.right();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if in_sidebar {
                let len = app.sidebar_entries.len();
                let idx = app.active_sidebar_index;
                if idx + 1 < len && !matches!(app.sidebar_entries[idx + 1], SidebarEntry::All) {
                    app.active_sidebar_index = idx + 1;
                }
            } else {
                app.select_next_task();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if in_sidebar {
                if app.active_sidebar_index > 0 {
                    app.active_sidebar_index -= 1;
                }
            } else {
                app.select_prev_task();
            }
        }
        KeyCode::Char('J') => {
            if in_sidebar {
                app.swap_list_block_down()?;
            } else {
                return actions::reorder_down(app);
            }
        }
        KeyCode::Char('K') => {
            if in_sidebar {
                app.swap_list_block_up()?;
            } else {
                return actions::reorder_up(app);
            }
        }
        KeyCode::Char('H') if !in_sidebar => {
            let ids = app.visual_selected_task_ids();
            let prior_anchor = match &app.mode {
                Mode::Visual { anchor } => Some(*anchor),
                _ => None,
            };
            enter_carry(app, ids, prior_anchor);
        }
        KeyCode::Char('>') => {
            let ids = app.visual_selected_task_ids();
            if !ids.is_empty() {
                return actions::send_to_next_list(app, &ids);
            }
        }
        KeyCode::Char('<') => {
            let ids = app.visual_selected_task_ids();
            if !ids.is_empty() {
                return actions::send_to_prev_list(app, &ids);
            }
        }
        KeyCode::Char('[') => {
            app.select_first_task_absolute();
        }
        KeyCode::Char(']') => {
            app.select_last_task();
        }
        KeyCode::Char('{') if !in_sidebar => {
            return actions::move_to_top(app);
        }
        KeyCode::Char('}') if !in_sidebar => {
            return actions::move_to_bottom(app);
        }
        KeyCode::Char('m') => {
            let ids = app.visual_selected_task_ids();
            let prior_anchor = match &app.mode {
                Mode::Visual { anchor } => Some(*anchor),
                _ => None,
            };
            enter_carry(app, ids, prior_anchor);
        }
        KeyCode::Char('x') => {
            if in_sidebar {
                let names = app.selected_sublist_names();
                if !names.is_empty() {
                    app.mode = Mode::ConfirmDelete {
                        scope: DeleteScope::Lists(names),
                    };
                }
            } else {
                let ids = app.visual_selected_task_ids();
                if !ids.is_empty() {
                    app.mode = Mode::ConfirmDelete {
                        scope: DeleteScope::Items(ids),
                    };
                }
            }
        }
        KeyCode::Char('t') => {
            if in_sidebar {
                let names = app.selected_sublist_names();
                if !names.is_empty() {
                    enter_tag_picker(app, TagTarget::Lists(names));
                }
            } else {
                let ids = app.visual_selected_task_ids();
                if !ids.is_empty() {
                    enter_tag_picker(app, TagTarget::Items(ids));
                }
            }
        }
        KeyCode::Char('g') => {
            app.select_first_task_absolute();
        }
        KeyCode::Char('G') => {
            app.select_last_task();
        }
        KeyCode::Char('q') => return Ok(SideEffect::Quit),
        _ => {}
    }
    Ok(SideEffect::None)
}

fn handle_add_form_key(app: &mut TuiApp, key: KeyEvent) -> Result<SideEffect, AppError> {
    match key.code {
        KeyCode::Enter => return actions::submit_add_form(app),
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Tab => {
            if let Mode::AddForm { list, .. } = &mut app.mode {
                let lists = app.adapter.lists();
                let names: Vec<&str> = lists.iter().map(|l| l.name.as_str()).collect();
                if let Some(pos) = names.iter().position(|n| *n == list.as_str()) {
                    *list = names[(pos + 1) % names.len()].to_string();
                }
            }
        }
        KeyCode::BackTab => {
            if let Mode::AddForm { list, .. } = &mut app.mode {
                let lists = app.adapter.lists();
                let names: Vec<&str> = lists.iter().map(|l| l.name.as_str()).collect();
                if let Some(pos) = names.iter().position(|n| *n == list.as_str()) {
                    *list = names[(pos + names.len() - 1) % names.len()].to_string();
                }
            }
        }
        KeyCode::Backspace => {
            if let Mode::AddForm { title, .. } = &mut app.mode {
                title.pop();
            }
        }
        KeyCode::Char(c) => {
            if let Mode::AddForm { title, .. } = &mut app.mode {
                title.push(c);
            }
        }
        _ => {}
    }
    Ok(SideEffect::None)
}

fn handle_add_sublist_key(app: &mut TuiApp, key: KeyEvent) -> Result<SideEffect, AppError> {
    match key.code {
        KeyCode::Enter => return actions::submit_add_sublist(app),
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Backspace => {
            if let Mode::AddSublist { name, .. } = &mut app.mode {
                name.pop();
            }
        }
        KeyCode::Char(c) if !c.is_whitespace() || c == '-' || c == '_' => {
            if let Mode::AddSublist { name, .. } = &mut app.mode {
                name.push(c);
            }
        }
        _ => {}
    }
    Ok(SideEffect::None)
}

fn handle_confirm_delete_key(app: &mut TuiApp, key: KeyEvent) -> Result<SideEffect, AppError> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Enter => actions::confirm_delete(app),
        _ => {
            app.mode = Mode::Normal;
            Ok(SideEffect::None)
        }
    }
}

fn handle_search_key(app: &mut TuiApp, key: KeyEvent) -> Result<SideEffect, AppError> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            app.select_search_result();
        }
        KeyCode::Down | KeyCode::Tab => {
            if let Mode::Search {
                results,
                list_state,
                ..
            } = &mut app.mode
            {
                let count = results.len();
                if count > 0 {
                    let current = list_state.selected().unwrap_or(0);
                    let next = if current + 1 >= count { 0 } else { current + 1 };
                    list_state.select(Some(next));
                }
            }
        }
        KeyCode::Up | KeyCode::BackTab => {
            if let Mode::Search {
                results,
                list_state,
                ..
            } = &mut app.mode
            {
                let count = results.len();
                if count > 0 {
                    let current = list_state.selected().unwrap_or(0);
                    let prev = if current == 0 { count - 1 } else { current - 1 };
                    list_state.select(Some(prev));
                }
            }
        }
        KeyCode::Backspace => {
            if let Mode::Search { query, .. } = &mut app.mode {
                query.pop();
            }
            app.update_search_results();
        }
        KeyCode::Char(c) => {
            if let Mode::Search { query, .. } = &mut app.mode {
                query.push(c);
            }
            app.update_search_results();
        }
        _ => {}
    }
    Ok(SideEffect::None)
}

fn handle_carry_to_list_key(app: &mut TuiApp, key: KeyEvent) -> Result<SideEffect, AppError> {
    match key.code {
        KeyCode::Esc => return actions::cancel_carry(app),

        KeyCode::Char('j') | KeyCode::Down => {
            let len = app.sidebar_entries.len();
            let idx = app.active_sidebar_index;
            if idx + 1 < len && !matches!(app.sidebar_entries[idx + 1], SidebarEntry::All) {
                app.active_sidebar_index = idx + 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up if app.active_sidebar_index > 0 => {
            app.active_sidebar_index -= 1;
        }

        KeyCode::Char('L') | KeyCode::Enter => {
            if let Some(SidebarEntry::List(name)) =
                app.sidebar_entries.get(app.active_sidebar_index).cloned()
            {
                return actions::drop_carry(app, &name);
            }
        }

        _ => {}
    }
    Ok(SideEffect::None)
}

fn handle_tag_picker_key(app: &mut TuiApp, key: KeyEvent) -> Result<SideEffect, AppError> {
    let tags = app.all_tags();
    let total = tags.len();
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => return actions::submit_tag_picker(app),
        KeyCode::Down => {
            if let Mode::TagPicker { cursor, .. } = &mut app.mode
                && total > 0
                && *cursor + 1 < total
            {
                *cursor += 1;
            }
        }
        KeyCode::Up => {
            if let Mode::TagPicker { cursor, .. } = &mut app.mode
                && *cursor > 0
            {
                *cursor -= 1;
            }
        }
        KeyCode::Char(' ') => {
            if total == 0 {
                return Ok(SideEffect::None);
            }
            if let Mode::TagPicker {
                cursor, selected, ..
            } = &mut app.mode
                && let Some(tag) = tags.get(*cursor).cloned()
            {
                if let Some(pos) = selected.iter().position(|t| t == &tag) {
                    selected.remove(pos);
                } else {
                    selected.push(tag);
                }
            }
        }
        KeyCode::Backspace => {
            if let Mode::TagPicker { new_tag, .. } = &mut app.mode {
                new_tag.pop();
            }
        }
        KeyCode::Char(c) if c.is_ascii_alphanumeric() || c == '-' || c == '_' => {
            if let Mode::TagPicker { new_tag, .. } = &mut app.mode {
                new_tag.push(c);
            }
        }
        _ => {}
    }
    Ok(SideEffect::None)
}

fn handle_find_key(app: &mut TuiApp, key: KeyEvent) -> Result<SideEffect, AppError> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            app.select_find_result();
        }
        KeyCode::Down | KeyCode::Tab => {
            if let Mode::Find {
                results,
                list_state,
                ..
            } = &mut app.mode
            {
                let count = results.len();
                if count > 0 {
                    let current = list_state.selected().unwrap_or(0);
                    let next = if current + 1 >= count { 0 } else { current + 1 };
                    list_state.select(Some(next));
                }
            }
        }
        KeyCode::Up | KeyCode::BackTab => {
            if let Mode::Find {
                results,
                list_state,
                ..
            } = &mut app.mode
            {
                let count = results.len();
                if count > 0 {
                    let current = list_state.selected().unwrap_or(0);
                    let prev = if current == 0 { count - 1 } else { current - 1 };
                    list_state.select(Some(prev));
                }
            }
        }
        KeyCode::Backspace => {
            if let Mode::Find { query, .. } = &mut app.mode {
                query.pop();
            }
            app.update_find_results();
        }
        KeyCode::Char(c) => {
            if let Mode::Find { query, .. } = &mut app.mode {
                query.push(c);
            }
            app.update_find_results();
        }
        _ => {}
    }
    Ok(SideEffect::None)
}
