use crate::app::app_error::AppError;

use super::app_state::{
    DeleteScope, FocusedPanel, ListFilter, Mode, SidebarEntry, TagTarget, TuiApp,
};

pub enum SideEffect {
    None,
    Quit,
    SuspendForEditor { task_id: String },
}

/// Reorder: move the selected item/block DOWN by one position.
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

    if !all_same {
        let to_move: Vec<String> = (start..=end)
            .filter(|&i| items[i].list != bottom_list)
            .map(|i| items[i].ext_id.clone())
            .collect();
        for id in &to_move {
            app.persist_move_to_bottom(id, &bottom_list)?;
        }
        app.refresh()?;
        let mut order: Vec<String> = list_items_in_order(app, &bottom_list);
        order.retain(|id| !to_move.contains(id));
        for (i, id) in to_move.iter().enumerate() {
            order.insert(i, id.clone());
        }
        app.persist_list_item_order(&bottom_list, &order)?;
        app.refresh()?;
        if !matches!(app.active_filter(), ListFilter::All) {
            app.jump_to_list(&bottom_list);
        }
        select_items_by_id(app, &sel_ids, was_visual);
        return Ok(SideEffect::None);
    }

    let list_ids: Vec<String> = items
        .iter()
        .filter(|it| it.list == bottom_list)
        .map(|it| it.ext_id.clone())
        .collect();
    let sel_at_bottom = list_ids.last().map(|s| s.as_str()) == Some(&items[end].ext_id);

    if sel_at_bottom {
        let Some(target) = app.next_list_for(&bottom_list) else {
            return Ok(SideEffect::None);
        };
        for id in &sel_ids {
            app.persist_move_to_bottom(id, &target)?;
        }
        app.refresh()?;
        let mut order: Vec<String> = list_items_in_order(app, &target);
        order.retain(|id| !sel_ids.contains(id));
        for (i, id) in sel_ids.iter().enumerate() {
            order.insert(i, id.clone());
        }
        app.persist_list_item_order(&target, &order)?;
        app.refresh()?;
        if !matches!(app.active_filter(), ListFilter::All) {
            app.jump_to_list(&target);
        }
        select_items_by_id(app, &sel_ids, was_visual);
        return Ok(SideEffect::None);
    }

    let mut order = list_ids;
    let fstart = order
        .iter()
        .position(|id| sel_ids.contains(id))
        .unwrap_or(0);
    let fend = fstart + sel_ids.len() - 1;
    if fend + 1 < order.len() {
        let below = order.remove(fend + 1);
        order.insert(fstart, below);
        app.persist_list_item_order(&bottom_list, &order)?;
    }
    app.refresh()?;
    app.task_list_state.select(Some(end + 1));
    if let Mode::Visual { anchor } = &mut app.mode {
        *anchor = start + 1;
    }
    Ok(SideEffect::None)
}

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

    if !all_same {
        let to_move: Vec<String> = (start..=end)
            .filter(|&i| items[i].list != top_list)
            .map(|i| items[i].ext_id.clone())
            .collect();
        for id in &to_move {
            app.persist_move_to_bottom(id, &top_list)?;
        }
        app.refresh()?;
        let mut order: Vec<String> = list_items_in_order(app, &top_list);
        order.retain(|id| !to_move.contains(id));
        order.extend(to_move);
        app.persist_list_item_order(&top_list, &order)?;
        app.refresh()?;
        if !matches!(app.active_filter(), ListFilter::All) {
            app.jump_to_list(&top_list);
        }
        select_items_by_id(app, &sel_ids, was_visual);
        return Ok(SideEffect::None);
    }

    let list_ids: Vec<String> = items
        .iter()
        .filter(|it| it.list == top_list)
        .map(|it| it.ext_id.clone())
        .collect();
    let sel_at_top = list_ids.first().map(|s| s.as_str()) == Some(&items[start].ext_id);

    if sel_at_top {
        let Some(target) = app.prev_list_for(&top_list) else {
            return Ok(SideEffect::None);
        };
        for id in &sel_ids {
            app.persist_move_to_bottom(id, &target)?;
        }
        app.refresh()?;
        let mut order: Vec<String> = list_items_in_order(app, &target);
        order.retain(|id| !sel_ids.contains(id));
        order.extend(sel_ids.iter().cloned());
        app.persist_list_item_order(&target, &order)?;
        app.refresh()?;
        if !matches!(app.active_filter(), ListFilter::All) {
            app.jump_to_list(&target);
        }
        select_items_by_id(app, &sel_ids, was_visual);
        return Ok(SideEffect::None);
    }

    let mut order = list_ids;
    let fstart = order
        .iter()
        .position(|id| sel_ids.contains(id))
        .unwrap_or(0);
    let fend = fstart + sel_ids.len() - 1;
    if fstart > 0 {
        let above = order.remove(fstart - 1);
        order.insert(fend, above);
        app.persist_list_item_order(&top_list, &order)?;
    }
    app.refresh()?;
    app.task_list_state.select(Some(end - 1));
    if let Mode::Visual { anchor } = &mut app.mode {
        *anchor = start - 1;
    }
    Ok(SideEffect::None)
}

pub fn move_to_top(app: &mut TuiApp) -> Result<SideEffect, AppError> {
    move_to_edge(app, true)
}

pub fn move_to_bottom(app: &mut TuiApp) -> Result<SideEffect, AppError> {
    move_to_edge(app, false)
}

fn move_to_edge(app: &mut TuiApp, to_top: bool) -> Result<SideEffect, AppError> {
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
    if to_top {
        for (i, id) in sel_ids.iter().enumerate() {
            order.insert(i, id.clone());
        }
    } else {
        order.extend(sel_ids.iter().cloned());
    }
    let was_visual = matches!(app.mode, Mode::Visual { .. });
    app.persist_list_item_order(&sel_list, &order)?;
    app.refresh()?;
    select_items_by_id(app, &sel_ids, was_visual);
    Ok(SideEffect::None)
}

pub fn send_to_next_list(app: &mut TuiApp, ids: &[String]) -> Result<SideEffect, AppError> {
    send_to_adjacent(app, ids, true)
}

pub fn send_to_prev_list(app: &mut TuiApp, ids: &[String]) -> Result<SideEffect, AppError> {
    send_to_adjacent(app, ids, false)
}

fn send_to_adjacent(
    app: &mut TuiApp,
    ids: &[String],
    forward: bool,
) -> Result<SideEffect, AppError> {
    if ids.is_empty() {
        return Ok(SideEffect::None);
    }
    let item_list = app
        .items
        .iter()
        .find(|i| i.ext_id == ids[0])
        .map(|i| i.list.clone());
    let Some(item_list) = item_list else {
        return Ok(SideEffect::None);
    };
    let target = if forward {
        app.next_list_for(&item_list)
    } else {
        app.prev_list_for(&item_list)
    };
    let Some(target) = target else {
        return Ok(SideEffect::None);
    };
    let was_visual = matches!(app.mode, Mode::Visual { .. });
    let id_list: Vec<String> = ids.to_vec();
    for id in ids {
        app.persist_move_to_bottom(id, &target)?;
    }
    app.refresh()?;
    if !matches!(app.active_filter(), ListFilter::All) {
        app.jump_to_list(&target);
    }
    select_items_by_id(app, &id_list, was_visual);
    app.set_status(format!("Sent {} item(s) to {target}", id_list.len()));
    Ok(SideEffect::None)
}

pub fn drop_carry(app: &mut TuiApp, target_list: &str) -> Result<SideEffect, AppError> {
    let (selected_ids, prior_anchor, pending_list_delete) = match &app.mode {
        Mode::CarryToList {
            selected_ids,
            prior_anchor,
            pending_list_delete,
            ..
        } => (
            selected_ids.clone(),
            *prior_anchor,
            pending_list_delete.clone(),
        ),
        _ => return Ok(SideEffect::None),
    };

    if selected_ids.is_empty() && pending_list_delete.is_empty() {
        app.mode = Mode::Normal;
        return Ok(SideEffect::None);
    }

    for id in &selected_ids {
        app.persist_move_to_bottom(id, target_list)?;
    }

    let mut deleted_lists: Vec<String> = Vec::new();
    if !pending_list_delete.is_empty() {
        // Items have been moved out; the lists are now empty.
        for name in &pending_list_delete {
            if target_list == name {
                continue;
            }
            app.adapter.delete_list(name)?;
            app.sidebar_entries
                .retain(|e| !matches!(e, SidebarEntry::List(n) if n == name));
            deleted_lists.push(name.clone());
        }
    }

    app.refresh()?;
    app.jump_to_list(target_list);
    app.focused_panel = FocusedPanel::TaskList;

    let count = selected_ids.len();
    let was_visual = prior_anchor.is_some() && deleted_lists.is_empty();
    select_items_by_id(app, &selected_ids, was_visual);
    if !was_visual {
        app.mode = Mode::Normal;
    }
    let status = if deleted_lists.is_empty() {
        format!("Moved {count} item(s) to {target_list}")
    } else {
        format!(
            "Moved {count} item(s) to {target_list}, deleted {} list(s)",
            deleted_lists.len()
        )
    };
    app.set_status(status);
    Ok(SideEffect::None)
}

pub fn cancel_carry(app: &mut TuiApp) -> Result<SideEffect, AppError> {
    if let Mode::CarryToList { prior_anchor, .. } = &app.mode {
        let anchor = *prior_anchor;
        match anchor {
            Some(a) => app.mode = Mode::Visual { anchor: a },
            None => app.mode = Mode::Normal,
        }
        app.focused_panel = FocusedPanel::TaskList;
        app.set_status("Cancelled carry");
    }
    Ok(SideEffect::None)
}

pub fn renormalize(app: &mut TuiApp) -> Result<SideEffect, AppError> {
    let (lists, items) = app.adapter.renormalize()?;
    app.refresh()?;
    app.set_status(format!("Renormalized {lists} list(s), {items} item(s)"));
    Ok(SideEffect::None)
}

fn list_items_in_order(app: &TuiApp, list: &str) -> Vec<String> {
    let mut items: Vec<&crate::adapter::Item> =
        app.items.iter().filter(|it| it.list == list).collect();
    items.sort_by(|a, b| {
        a.order
            .partial_cmp(&b.order)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    items.into_iter().map(|i| i.ext_id.clone()).collect()
}

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

pub fn confirm_delete(app: &mut TuiApp) -> Result<SideEffect, AppError> {
    let scope = match &app.mode {
        Mode::ConfirmDelete { scope } => scope.clone(),
        _ => return Ok(SideEffect::None),
    };

    match scope {
        DeleteScope::Items(ids) => {
            for id in &ids {
                app.adapter.delete_item(id)?;
            }
            app.mode = Mode::Normal;
            app.refresh()?;
            app.set_status(format!("Deleted {} item(s)", ids.len()));
            Ok(SideEffect::None)
        }
        DeleteScope::Lists(names) => {
            // Partition into empty (delete now) and non-empty (need carry).
            let occupied_items: Vec<String> = app.item_ids_in_lists(&names);

            if occupied_items.is_empty() {
                let count = names.len();
                for name in &names {
                    app.adapter.delete_list(name)?;
                    app.sidebar_entries
                        .retain(|e| !matches!(e, SidebarEntry::List(n) if n == name));
                }
                app.refresh()?;
                if app.active_sidebar_index >= app.sidebar_entries.len() {
                    app.active_sidebar_index = app.sidebar_entries.len().saturating_sub(1);
                }
                app.mode = Mode::Normal;
                app.set_status(format!("Deleted {count} list(s)"));
                Ok(SideEffect::None)
            } else {
                // Non-empty: enter carry with the items, then delete on drop.
                let mut source_lists: Vec<String> = occupied_items
                    .iter()
                    .filter_map(|id| app.items.iter().find(|it| it.ext_id == *id))
                    .map(|it| it.list.clone())
                    .collect();
                source_lists.sort();
                source_lists.dedup();
                let count = occupied_items.len();
                let list_count = names.len();
                app.mode = Mode::CarryToList {
                    selected_ids: occupied_items,
                    source_lists,
                    prior_anchor: None,
                    pending_list_delete: names,
                };
                app.focused_panel = FocusedPanel::Sidebar;
                app.set_status(format!(
                    "Move {count} item(s) out of {list_count} list(s) — L/Enter to drop & delete, Esc to cancel"
                ));
                Ok(SideEffect::None)
            }
        }
    }
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

    let bottom_key = app.bottom_of_list_with_renorm(&list)?;
    let (new_item, _) = app.adapter.create_item(None, &title, "", bottom_key)?;
    let new_id = new_item.ext_id.clone();
    app.refresh()?;

    let mut order_ids: Vec<String> = list_items_in_order(app, &list);
    if let Some(pos) = order_ids.iter().position(|id| *id == new_id) {
        order_ids.remove(pos);
    }
    let clamped = insert_at.min(order_ids.len());
    order_ids.insert(clamped, new_id.clone());
    app.persist_list_item_order(&list, &order_ids)?;
    app.refresh()?;

    app.mode = Mode::Normal;
    if !matches!(app.active_filter(), ListFilter::All) {
        app.jump_to_list(&list);
    }
    let pos_in_view = app
        .current_items()
        .iter()
        .position(|i| i.ext_id == new_id)
        .unwrap_or(0);
    app.task_list_state.select(Some(pos_in_view));
    app.set_status(format!("Added: {title}"));
    Ok(SideEffect::None)
}

pub fn submit_tag_picker(app: &mut TuiApp) -> Result<SideEffect, AppError> {
    let (target, mut selected, initial, new_tag) = match &app.mode {
        Mode::TagPicker {
            target,
            selected,
            initial,
            new_tag,
            ..
        } => (
            target.clone(),
            selected.clone(),
            initial.clone(),
            new_tag.trim().to_string(),
        ),
        _ => return Ok(SideEffect::None),
    };

    if !new_tag.is_empty() && !selected.iter().any(|t| t == &new_tag) {
        selected.push(new_tag);
    }
    selected.sort();
    selected.dedup();

    let to_add: Vec<String> = selected
        .iter()
        .filter(|t| !initial.contains(t))
        .cloned()
        .collect();
    let to_remove: Vec<String> = initial
        .iter()
        .filter(|t| !selected.contains(t))
        .cloned()
        .collect();

    if to_add.is_empty() && to_remove.is_empty() {
        app.mode = Mode::Normal;
        return Ok(SideEffect::None);
    }

    let count = match &target {
        TagTarget::Items(ids) => {
            for id in ids {
                let existing = app.adapter.find_item(id)?;
                let merged = merge_tags(&existing.tags, &to_add, &to_remove);
                app.adapter.set_item_tags(id, &merged)?;
            }
            ids.len()
        }
        TagTarget::Lists(names) => {
            let lists = app.adapter.lists();
            for name in names {
                let existing = lists
                    .iter()
                    .find(|l| &l.name == name)
                    .map(|l| l.tags.clone())
                    .unwrap_or_default();
                let merged = merge_tags(&existing, &to_add, &to_remove);
                app.adapter.set_list_tags(name, &merged)?;
            }
            names.len()
        }
    };
    app.refresh()?;
    app.mode = Mode::Normal;
    let label = match target {
        TagTarget::Items(_) => "item(s)",
        TagTarget::Lists(_) => "list(s)",
    };
    app.set_status(format!(
        "Tagged {count} {label} (+{}, -{})",
        to_add.len(),
        to_remove.len()
    ));
    Ok(SideEffect::None)
}

fn merge_tags(existing: &[String], to_add: &[String], to_remove: &[String]) -> Vec<String> {
    let mut combined: Vec<String> = existing
        .iter()
        .filter(|t| !to_remove.contains(t))
        .cloned()
        .collect();
    for t in to_add {
        if !combined.iter().any(|x| x == t) {
            combined.push(t.clone());
        }
    }
    combined.sort();
    combined.dedup();
    combined
}

pub fn submit_add_sublist(app: &mut TuiApp) -> Result<SideEffect, AppError> {
    let (name, insert_at) = match &app.mode {
        Mode::AddSublist { name, insert_at } => (name.trim().to_string(), *insert_at),
        _ => return Ok(SideEffect::None),
    };

    if name.is_empty() {
        app.mode = Mode::Normal;
        return Ok(SideEffect::None);
    }

    app.create_sublist(&name, insert_at)?;
    app.mode = Mode::Normal;
    app.refresh()?;
    app.set_status(format!("Created list: {name}"));
    Ok(SideEffect::None)
}
