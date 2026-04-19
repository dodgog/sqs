use crate::adapters::markdown_todolists::identity;
use crate::app::app_error::AppError;

use super::app_state::{ListFilter, Mode, TuiApp};

pub enum SideEffect {
    None,
    Quit,
    SuspendForEditor { task_id: String },
}

pub fn move_to_list(app: &mut TuiApp, target_list: &str) -> Result<SideEffect, AppError> {
    let Some(item) = app.selected_item() else {
        return Ok(SideEffect::None);
    };
    if item.list == target_list {
        app.set_status(format!("{} is already in {target_list}", item.ext_id));
        return Ok(SideEffect::None);
    }
    let ext_id = item.ext_id.clone();
    app.adapter.move_item(&ext_id, target_list)?;
    app.refresh()?;
    app.jump_to_list(target_list);
    focus_item(app, &ext_id);
    app.set_status(format!("Moved {ext_id} to {target_list}"));
    Ok(SideEffect::None)
}

pub fn move_items_to_list(
    app: &mut TuiApp,
    item_ids: &[String],
    target_list: &str,
) -> Result<SideEffect, AppError> {
    let mut moved = 0;
    for id in item_ids {
        let item = app.adapter.find_item(id)?;
        if item.list != target_list {
            app.adapter.move_item(id, target_list)?;
            moved += 1;
        }
    }
    app.refresh()?;
    app.jump_to_list(target_list);
    app.set_status(format!("Moved {moved} item(s) to {target_list}"));
    Ok(SideEffect::None)
}

/// Reorder: move the selected item/block DOWN by one position.
/// Multi-list selection: consolidates first (moves top items into bottom list).
/// At list boundary: uses sidebar order (respects empty lists).
pub fn reorder_down(app: &mut TuiApp) -> Result<SideEffect, AppError> {
    let (start, end) = selection_range(app);
    let items = app.current_items();
    if items.is_empty() {
        return Ok(SideEffect::None);
    }

    let was_visual = matches!(app.mode, Mode::Visual { .. });
    let sel_ids: Vec<String> = (start..=end)
        .filter_map(|i| items.get(i).map(|it| it.ext_id.clone()))
        .collect();
    let bottom_list = items[end].list.clone();
    let top_list = items[start].list.clone();
    let all_same = top_list == bottom_list;

    // 1. Multi-list selection: consolidate into bottom list
    if !all_same {
        let to_move: Vec<String> = (start..=end)
            .filter(|&i| items[i].list != bottom_list)
            .map(|i| items[i].ext_id.clone())
            .collect();
        for id in &to_move {
            app.adapter.move_item(id, &bottom_list)?;
        }
        // Place consolidated items at top of bottom list
        app.refresh()?;
        let mut order: Vec<String> = app
            .items
            .iter()
            .filter(|it| it.list == bottom_list)
            .map(|it| it.ext_id.clone())
            .collect();
        order.retain(|id| !to_move.contains(id));
        for (i, id) in to_move.iter().enumerate() {
            order.insert(i, id.clone());
        }
        app.adapter.reorder_items(&bottom_list, &order)?;
        app.refresh()?;
        if !matches!(app.active_filter(), ListFilter::All) {
            app.jump_to_list(&bottom_list);
        }
        select_items_by_id(app, &sel_ids, was_visual);
        return Ok(SideEffect::None);
    }

    // 2. All same list — check if at bottom
    let list_ids: Vec<String> = items
        .iter()
        .filter(|it| it.list == bottom_list)
        .map(|it| it.ext_id.clone())
        .collect();
    let sel_at_bottom = list_ids.last().map(|s| s.as_str()) == Some(&items[end].ext_id);

    if sel_at_bottom {
        // Move to next list in sidebar order (respects empty lists)
        let Some(target) = app.next_list_for(&bottom_list) else {
            return Ok(SideEffect::None);
        };
        for id in &sel_ids {
            app.adapter.move_item(id, &target)?;
        }
        // Place at top of target
        app.refresh()?;
        let mut order: Vec<String> = app
            .items
            .iter()
            .filter(|it| it.list == target)
            .map(|it| it.ext_id.clone())
            .collect();
        order.retain(|id| !sel_ids.contains(id));
        for (i, id) in sel_ids.iter().enumerate() {
            order.insert(i, id.clone());
        }
        app.adapter.reorder_items(&target, &order)?;
        app.refresh()?;
        if !matches!(app.active_filter(), ListFilter::All) {
            app.jump_to_list(&target);
        }
        select_items_by_id(app, &sel_ids, was_visual);
        return Ok(SideEffect::None);
    }

    // 3. Within list — normal swap
    let mut order = list_ids;
    let fstart = order
        .iter()
        .position(|id| sel_ids.contains(id))
        .unwrap_or(0);
    let fend = fstart + sel_ids.len() - 1;
    if fend + 1 < order.len() {
        let below = order.remove(fend + 1);
        order.insert(fstart, below);
        app.adapter.reorder_items(&bottom_list, &order)?;
    }
    app.refresh()?;
    app.task_list_state.select(Some(end + 1));
    if let Mode::Visual { anchor } = &mut app.mode {
        *anchor = start + 1;
    }
    Ok(SideEffect::None)
}

/// Reorder: move the selected item/block UP by one position.
/// Multi-list selection: consolidates first (moves bottom items into top list).
/// At list boundary: uses sidebar order (respects empty lists).
pub fn reorder_up(app: &mut TuiApp) -> Result<SideEffect, AppError> {
    let (start, end) = selection_range(app);
    let items = app.current_items();
    if items.is_empty() {
        return Ok(SideEffect::None);
    }

    let was_visual = matches!(app.mode, Mode::Visual { .. });
    let sel_ids: Vec<String> = (start..=end)
        .filter_map(|i| items.get(i).map(|it| it.ext_id.clone()))
        .collect();
    let top_list = items[start].list.clone();
    let bottom_list = items[end].list.clone();
    let all_same = top_list == bottom_list;

    // 1. Multi-list selection: consolidate into top list
    if !all_same {
        let to_move: Vec<String> = (start..=end)
            .filter(|&i| items[i].list != top_list)
            .map(|i| items[i].ext_id.clone())
            .collect();
        for id in &to_move {
            app.adapter.move_item(id, &top_list)?;
        }
        // Place consolidated items at bottom of top list
        app.refresh()?;
        let mut order: Vec<String> = app
            .items
            .iter()
            .filter(|it| it.list == top_list)
            .map(|it| it.ext_id.clone())
            .collect();
        order.retain(|id| !to_move.contains(id));
        order.extend(to_move);
        app.adapter.reorder_items(&top_list, &order)?;
        app.refresh()?;
        if !matches!(app.active_filter(), ListFilter::All) {
            app.jump_to_list(&top_list);
        }
        select_items_by_id(app, &sel_ids, was_visual);
        return Ok(SideEffect::None);
    }

    // 2. All same list — check if at top
    let list_ids: Vec<String> = items
        .iter()
        .filter(|it| it.list == top_list)
        .map(|it| it.ext_id.clone())
        .collect();
    let sel_at_top = list_ids.first().map(|s| s.as_str()) == Some(&items[start].ext_id);

    if sel_at_top {
        // Move to prev list in sidebar order
        let Some(target) = app.prev_list_for(&top_list) else {
            return Ok(SideEffect::None);
        };
        for id in &sel_ids {
            app.adapter.move_item(id, &target)?;
        }
        // Place at bottom of target
        app.refresh()?;
        let mut order: Vec<String> = app
            .items
            .iter()
            .filter(|it| it.list == target)
            .map(|it| it.ext_id.clone())
            .collect();
        order.retain(|id| !sel_ids.contains(id));
        order.extend(sel_ids.iter().cloned());
        app.adapter.reorder_items(&target, &order)?;
        app.refresh()?;
        if !matches!(app.active_filter(), ListFilter::All) {
            app.jump_to_list(&target);
        }
        select_items_by_id(app, &sel_ids, was_visual);
        return Ok(SideEffect::None);
    }

    // 3. Within list — normal swap
    let mut order = list_ids;
    let fstart = order
        .iter()
        .position(|id| sel_ids.contains(id))
        .unwrap_or(0);
    let fend = fstart + sel_ids.len() - 1;
    if fstart > 0 {
        let above = order.remove(fstart - 1);
        order.insert(fend, above);
        app.adapter.reorder_items(&top_list, &order)?;
    }
    app.refresh()?;
    app.task_list_state.select(Some(end - 1));
    if let Mode::Visual { anchor } = &mut app.mode {
        *anchor = start - 1;
    }
    Ok(SideEffect::None)
}

/// Move selected item(s) to the top of their list.
pub fn move_to_top(app: &mut TuiApp) -> Result<SideEffect, AppError> {
    let (start, end) = selection_range(app);
    let items = app.current_items();
    if items.is_empty() || start == 0 {
        return Ok(SideEffect::None);
    }

    let sel_list = items[start].list.clone();
    let sel_ids: Vec<String> = (start..=end)
        .filter_map(|i| items.get(i).map(|it| it.ext_id.clone()))
        .collect();

    let mut order: Vec<String> = items
        .iter()
        .filter(|it| it.list == sel_list)
        .map(|it| it.ext_id.clone())
        .collect();

    // Remove selected, prepend them
    order.retain(|id| !sel_ids.contains(id));
    for (i, id) in sel_ids.iter().enumerate() {
        order.insert(i, id.clone());
    }

    let was_visual = matches!(app.mode, Mode::Visual { .. });
    app.adapter.reorder_items(&sel_list, &order)?;
    app.refresh()?;
    select_items_by_id(app, &sel_ids, was_visual);
    Ok(SideEffect::None)
}

/// Move selected item(s) to the bottom of their list.
pub fn move_to_bottom(app: &mut TuiApp) -> Result<SideEffect, AppError> {
    let (start, end) = selection_range(app);
    let items = app.current_items();
    if items.is_empty() {
        return Ok(SideEffect::None);
    }

    let sel_list = items[start].list.clone();
    let sel_ids: Vec<String> = (start..=end)
        .filter_map(|i| items.get(i).map(|it| it.ext_id.clone()))
        .collect();

    let mut order: Vec<String> = items
        .iter()
        .filter(|it| it.list == sel_list)
        .map(|it| it.ext_id.clone())
        .collect();

    order.retain(|id| !sel_ids.contains(id));
    for id in &sel_ids {
        order.push(id.clone());
    }

    let was_visual = matches!(app.mode, Mode::Visual { .. });
    app.adapter.reorder_items(&sel_list, &order)?;
    app.refresh()?;
    select_items_by_id(app, &sel_ids, was_visual);
    Ok(SideEffect::None)
}

/// Send items to the next list, follow them.
pub fn send_to_next_list(app: &mut TuiApp, ids: &[String]) -> Result<SideEffect, AppError> {
    if ids.is_empty() {
        return Ok(SideEffect::None);
    }
    let item = app.adapter.find_item(&ids[0])?;
    let Some(target) = app.next_list_for(&item.list) else {
        return Ok(SideEffect::None);
    };
    let was_visual = matches!(app.mode, Mode::Visual { .. });
    let id_list: Vec<String> = ids.to_vec();
    for id in ids {
        app.adapter.move_item(id, &target)?;
    }
    app.refresh()?;
    if !matches!(app.active_filter(), ListFilter::All) {
        app.jump_to_list(&target);
    }
    select_items_by_id(app, &id_list, was_visual);
    app.set_status(format!("Sent {} item(s) to {target}", id_list.len()));
    Ok(SideEffect::None)
}

/// Send items to the previous list, follow them.
pub fn send_to_prev_list(app: &mut TuiApp, ids: &[String]) -> Result<SideEffect, AppError> {
    if ids.is_empty() {
        return Ok(SideEffect::None);
    }
    let item = app.adapter.find_item(&ids[0])?;
    let Some(target) = app.prev_list_for(&item.list) else {
        return Ok(SideEffect::None);
    };
    let was_visual = matches!(app.mode, Mode::Visual { .. });
    let id_list: Vec<String> = ids.to_vec();
    for id in ids {
        app.adapter.move_item(id, &target)?;
    }
    app.refresh()?;
    if !matches!(app.active_filter(), ListFilter::All) {
        app.jump_to_list(&target);
    }
    select_items_by_id(app, &id_list, was_visual);
    app.set_status(format!("Sent {} item(s) to {target}", id_list.len()));
    Ok(SideEffect::None)
}

/// After moving items, find them by ID in the current view and set selection.
fn select_items_by_id(app: &mut TuiApp, ids: &[String], was_visual: bool) {
    let items = app.current_items();
    let positions: Vec<usize> = ids
        .iter()
        .filter_map(|id| items.iter().position(|it| it.ext_id == *id))
        .collect();
    if let (Some(&first), Some(&last)) = (positions.iter().min(), positions.iter().max()) {
        app.task_list_state.select(Some(last));
        if was_visual {
            app.mode = Mode::Visual { anchor: first };
        }
    }
}

fn selection_range(app: &TuiApp) -> (usize, usize) {
    if let Some((s, e)) = app.visual_selection_range() {
        (s, e)
    } else {
        let c = app.task_list_state.selected().unwrap_or(0);
        (c, c)
    }
}

fn focus_item(app: &mut TuiApp, ext_id: &str) {
    let items = app.current_items();
    if let Some(pos) = items.iter().position(|i| i.ext_id == ext_id) {
        app.task_list_state.select(Some(pos));
    }
}

pub fn confirm_delete(app: &mut TuiApp) -> Result<SideEffect, AppError> {
    let task_id = match &app.mode {
        Mode::ConfirmDelete { task_id } => task_id.clone(),
        _ => return Ok(SideEffect::None),
    };

    app.adapter.delete_item(&task_id)?;
    app.mode = Mode::Normal;
    app.refresh()?;
    app.set_status(format!("Deleted: {task_id}"));
    Ok(SideEffect::None)
}

pub fn submit_add_form(app: &mut TuiApp) -> Result<SideEffect, AppError> {
    let (title, list, insert_at) = match &app.mode {
        Mode::AddForm {
            title,
            list,
            insert_at,
        } => (title.trim().to_string(), list.clone(), *insert_at),
        _ => return Ok(SideEffect::None),
    };

    if title.is_empty() {
        app.mode = Mode::Normal;
        return Ok(SideEffect::None);
    }

    let existing = app.items.iter().map(|i| i.ext_id.clone()).collect();
    let id = identity::generate_id(&existing);
    let new_id = id.clone();
    let order = insert_at as f64;

    app.adapter
        .create_item(Some(&id), &list, &title, "", order)?;
    app.refresh()?;

    // Reorder to place at correct position
    let items = app.current_items();
    if items.len() > 1 {
        let mut order_ids: Vec<String> = items.iter().map(|i| i.ext_id.clone()).collect();
        if let Some(pos) = order_ids.iter().position(|id| *id == new_id) {
            order_ids.remove(pos);
        }
        let clamped = insert_at.min(order_ids.len());
        order_ids.insert(clamped, new_id.clone());
        app.adapter.reorder_items(&list, &order_ids)?;
        app.refresh()?;
    }

    app.mode = Mode::Normal;
    app.task_list_state.select(Some(
        insert_at.min(app.current_items().len().saturating_sub(1)),
    ));
    app.set_status(format!("Added: {title}"));
    Ok(SideEffect::None)
}
