use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;

use crate::app::app_error::AppError;

use super::actions::{self, SideEffect};
use super::app_state::{FocusedPanel, Mode, TuiApp};

/// Poll for a crossterm event, returning None on timeout.
pub fn poll_event(timeout: Duration) -> std::io::Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

/// Map a key event to state mutations on the app, given the current mode.
/// Returns a SideEffect that the main loop may need to handle.
pub fn handle_key(app: &mut TuiApp, key: KeyEvent) -> Result<SideEffect, AppError> {
    match &app.mode {
        Mode::Normal => handle_normal_key(app, key),
        Mode::Visual { .. } => handle_visual_key(app, key),
        Mode::AddForm { .. } => handle_add_form_key(app, key),
        Mode::ConfirmDelete { .. } => handle_confirm_delete_key(app, key),
        Mode::MoveTarget => handle_move_target_key(app, key),
        Mode::Search { .. } => handle_search_key(app, key),
    }
}

fn handle_normal_key(app: &mut TuiApp, key: KeyEvent) -> Result<SideEffect, AppError> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return Ok(SideEffect::Quit),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return Ok(SideEffect::Quit);
        }

        // Panel focus navigation
        KeyCode::Char('h') | KeyCode::Left => {
            app.focused_panel = app.focused_panel.left();
        }
        KeyCode::Char('l') | KeyCode::Right => {
            app.focused_panel = app.focused_panel.right();
        }
        KeyCode::Char(' ') => {
            app.focused_panel = match app.focused_panel {
                FocusedPanel::Sidebar => FocusedPanel::TaskList,
                FocusedPanel::TaskList => FocusedPanel::Sidebar,
                FocusedPanel::Detail => FocusedPanel::Sidebar,
            };
        }

        // Vertical navigation — depends on focused panel
        KeyCode::Char('j') | KeyCode::Down => match app.focused_panel {
            FocusedPanel::Sidebar => app.next_queue(),
            FocusedPanel::TaskList | FocusedPanel::Detail => app.select_next_task(),
        },
        KeyCode::Char('k') | KeyCode::Up => match app.focused_panel {
            FocusedPanel::Sidebar => app.prev_queue(),
            FocusedPanel::TaskList | FocusedPanel::Detail => app.select_prev_task(),
        },

        // Tab cycles queues regardless of panel focus
        KeyCode::Tab => app.next_queue(),
        KeyCode::BackTab => app.prev_queue(),

        // Direct queue jump (1-6) regardless of panel focus
        KeyCode::Char(c @ '1'..='6') => {
            let index = (c as usize) - ('1' as usize);
            app.select_queue_by_index(index);
        }

        // Task actions
        KeyCode::Char('m') if app.selected_item().is_some() => {
            app.mode = Mode::MoveTarget;
        }
        KeyCode::Char('x') => {
            if let Some(item) = app.selected_item() {
                app.mode = Mode::ConfirmDelete {
                    task_id: item.ext_id.clone(),
                };
            }
        }

        // Search
        KeyCode::Char('/') => {
            app.mode = Mode::Search {
                query: String::new(),
                results: Vec::new(),
                list_state: ListState::default(),
            };
            app.update_search_results();
        }

        // Add task: O/i=insert before cursor, o/a=append after cursor
        KeyCode::Char('O') | KeyCode::Char('i') => {
            let list = match app.active_filter() {
                crate::tui::app_state::ListFilter::Single(name) => name,
                crate::tui::app_state::ListFilter::All => app
                    .selected_item()
                    .map(|i| i.list.clone())
                    .unwrap_or_else(|| "inbox".to_string()),
            };
            let insert_at = app.task_list_state.selected().unwrap_or(0);
            app.mode = Mode::AddForm {
                title: String::new(),
                list,
                insert_at,
            };
        }
        KeyCode::Char('o') | KeyCode::Char('a') => {
            let list = match app.active_filter() {
                crate::tui::app_state::ListFilter::Single(name) => name,
                crate::tui::app_state::ListFilter::All => app
                    .selected_item()
                    .map(|i| i.list.clone())
                    .unwrap_or_else(|| "inbox".to_string()),
            };
            let insert_at = app.task_list_state.selected().map(|i| i + 1).unwrap_or(0);
            app.mode = Mode::AddForm {
                title: String::new(),
                list,
                insert_at,
            };
        }

        // Edit in $EDITOR
        KeyCode::Char('e') => {
            if let Some(item) = app.selected_item() {
                return Ok(SideEffect::SuspendForEditor {
                    task_id: item.ext_id.clone(),
                });
            }
        }

        // Top/bottom navigation
        KeyCode::Char('g') if app.focused_panel == FocusedPanel::TaskList => {
            app.select_first_task_absolute();
        }
        KeyCode::Char('G') if app.focused_panel == FocusedPanel::TaskList => {
            app.select_last_task();
        }

        // J/K — reorder item (or swap list in sidebar)
        KeyCode::Char('J') => match app.focused_panel {
            FocusedPanel::Sidebar => app.swap_list_down(),
            FocusedPanel::TaskList | FocusedPanel::Detail => {
                return actions::reorder_down(app);
            }
        },
        KeyCode::Char('K') => match app.focused_panel {
            FocusedPanel::Sidebar => app.swap_list_up(),
            FocusedPanel::TaskList | FocusedPanel::Detail => {
                return actions::reorder_up(app);
            }
        },

        // </> — switch to prev/next list
        KeyCode::Char('>') => app.next_queue(),
        KeyCode::Char('<') => app.prev_queue(),

        // [/] — jump cursor to first/last item
        KeyCode::Char('[') if app.focused_panel != FocusedPanel::Sidebar => {
            app.select_first_task_absolute();
        }
        KeyCode::Char(']') if app.focused_panel != FocusedPanel::Sidebar => {
            app.select_last_task();
        }

        // {/} — move selected item(s) to top/bottom of current list
        KeyCode::Char('{') if app.focused_panel != FocusedPanel::Sidebar => {
            return actions::move_to_top(app);
        }
        KeyCode::Char('}') if app.focused_panel != FocusedPanel::Sidebar => {
            return actions::move_to_bottom(app);
        }

        // Visual mode (items pane or sidebar)
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

        // Refresh
        KeyCode::Char('r') => {
            app.refresh()?;
            app.set_status("Refreshed");
        }

        _ => {}
    }
    Ok(SideEffect::None)
}

fn handle_visual_key(app: &mut TuiApp, key: KeyEvent) -> Result<SideEffect, AppError> {
    let in_sidebar = app.focused_panel == FocusedPanel::Sidebar;
    match key.code {
        KeyCode::Esc | KeyCode::Char('v') => {
            app.mode = Mode::Normal;
        }
        // Keep visual mode when switching panes
        KeyCode::Char('h') | KeyCode::Left => {
            app.focused_panel = app.focused_panel.left();
        }
        KeyCode::Char('l') | KeyCode::Right => {
            app.focused_panel = app.focused_panel.right();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if in_sidebar {
                let len = app.sidebar_entries.len();
                let idx = app.active_sidebar_index;
                if idx + 1 < len
                    && !matches!(
                        app.sidebar_entries[idx + 1],
                        crate::tui::app_state::SidebarEntry::All
                    )
                {
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
                app.swap_list_block_down();
            } else {
                return actions::reorder_down(app);
            }
        }
        KeyCode::Char('K') => {
            if in_sidebar {
                app.swap_list_block_up();
            } else {
                return actions::reorder_up(app);
            }
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
        KeyCode::Char('m') if !app.visual_selected_task_ids().is_empty() => {
            app.mode = Mode::MoveTarget;
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

fn handle_move_target_key(app: &mut TuiApp, key: KeyEvent) -> Result<SideEffect, AppError> {
    // Number keys 1-9 select by position in sidebar list
    let lists = app.adapter.lists();
    let target = match key.code {
        KeyCode::Char(c @ '1'..='9') => {
            let idx = (c as usize) - ('1' as usize);
            lists.get(idx).map(|l| l.name.clone())
        }
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            return Ok(SideEffect::None);
        }
        _ => {
            // Match first letter of list name
            let ch = match key.code {
                KeyCode::Char(c) => Some(c),
                _ => None,
            };
            ch.and_then(|c| {
                lists
                    .iter()
                    .find(|l| l.name.starts_with(c))
                    .map(|l| l.name.clone())
            })
        }
    };

    let Some(target) = target else {
        app.mode = Mode::Normal;
        return Ok(SideEffect::None);
    };

    let visual_ids = app.visual_selected_task_ids();
    app.mode = Mode::Normal;
    if visual_ids.is_empty() {
        actions::move_to_list(app, &target)
    } else {
        actions::move_items_to_list(app, &visual_ids, &target)
    }
}
