use std::fmt;
use std::time::Instant;

use ratatui::widgets::ListState;

use crate::adapter::{Adapter, Item, ListDef};
use crate::app::app_error::AppError;

/// What the sidebar can show: a named list or "all".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarEntry {
    List(String),
    All,
}

fn default_sidebar_entries() -> Vec<SidebarEntry> {
    vec![
        SidebarEntry::List("now".into()),
        SidebarEntry::List("next".into()),
        SidebarEntry::List("later".into()),
        SidebarEntry::List("inbox".into()),
        SidebarEntry::List("done".into()),
        SidebarEntry::All,
    ]
}

fn sidebar_from_adapter_lists(lists: &[ListDef]) -> Vec<SidebarEntry> {
    let mut entries: Vec<SidebarEntry> = lists
        .iter()
        .map(|l| SidebarEntry::List(l.name.clone()))
        .collect();
    entries.push(SidebarEntry::All);
    if entries.len() == 1 {
        return default_sidebar_entries();
    }
    entries
}

/// Which items to show in the item list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListFilter {
    Single(String),
    All,
}

impl fmt::Display for ListFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Single(name) => write!(f, "{name}"),
            Self::All => write!(f, "all"),
        }
    }
}

pub struct ListCounts {
    counts: std::collections::HashMap<String, usize>,
    pub total: usize,
}

impl ListCounts {
    pub fn get(&self, name: &str) -> usize {
        self.counts.get(name).copied().unwrap_or(0)
    }
}

/// How long status messages stay visible.
const STATUS_MESSAGE_TTL_SECS: u64 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPanel {
    Sidebar,
    TaskList,
    Detail,
}

impl FocusedPanel {
    pub fn left(self) -> Self {
        match self {
            Self::Detail => Self::TaskList,
            Self::TaskList => Self::Sidebar,
            Self::Sidebar => Self::Sidebar,
        }
    }

    pub fn right(self) -> Self {
        match self {
            Self::Sidebar => Self::TaskList,
            Self::TaskList => Self::Detail,
            Self::Detail => Self::Detail,
        }
    }
}

pub enum Mode {
    Normal,
    Visual {
        anchor: usize,
    },
    AddForm {
        title: String,
        list: String,
        /// Insert position in the current item list (before this index)
        insert_at: usize,
    },
    ConfirmDelete {
        task_id: String,
    },
    MoveTarget,
    Search {
        query: String,
        results: Vec<(String, String)>, // (ext_id, list_name)
        list_state: ListState,
    },
}

pub struct TuiApp {
    pub adapter: Box<dyn Adapter>,

    // Cached item data (from adapter.scan())
    pub items: Vec<Item>,

    // Sidebar
    pub sidebar_entries: Vec<SidebarEntry>,

    // Navigation
    pub active_sidebar_index: usize,
    pub task_list_state: ListState,

    // Panel focus
    pub focused_panel: FocusedPanel,
    pub detail_scroll: u16,

    // Mode
    pub mode: Mode,

    // Transient status message
    pub status_message: Option<(String, Instant)>,

    // Redraw flag — set when state changes, cleared after draw
    pub needs_redraw: bool,
}

impl TuiApp {
    pub fn new(adapter: Box<dyn Adapter>) -> Result<Self, AppError> {
        let items = adapter.scan()?;
        let sidebar_entries = sidebar_from_adapter_lists(&adapter.lists());
        let mut app = Self {
            adapter,
            items,
            sidebar_entries,
            active_sidebar_index: 0,
            task_list_state: ListState::default(),
            focused_panel: FocusedPanel::TaskList,
            detail_scroll: 0,
            mode: Mode::Normal,
            status_message: None,
            needs_redraw: true,
        };
        app.select_first_task();
        Ok(app)
    }

    pub fn refresh(&mut self) -> Result<(), AppError> {
        self.items = self.adapter.scan()?;
        let count = self.current_items().len();
        if count == 0 {
            self.task_list_state.select(None);
        } else if let Some(i) = self.task_list_state.selected()
            && i >= count
        {
            self.task_list_state.select(Some(count - 1));
        }
        Ok(())
    }

    pub fn sidebar_entries(&self) -> &[SidebarEntry] {
        &self.sidebar_entries
    }

    pub fn active_filter(&self) -> ListFilter {
        match &self.sidebar_entries[self.active_sidebar_index] {
            SidebarEntry::List(name) => ListFilter::Single(name.clone()),
            SidebarEntry::All => ListFilter::All,
        }
    }

    pub fn list_counts(&self) -> ListCounts {
        let mut counts = std::collections::HashMap::new();
        for item in &self.items {
            *counts.entry(item.list.clone()).or_insert(0) += 1;
        }
        let total = self.items.len();
        ListCounts { counts, total }
    }

    pub fn current_items(&self) -> Vec<&Item> {
        match self.active_filter() {
            ListFilter::Single(ref name) => self.items.iter().filter(|i| i.list == *name).collect(),
            ListFilter::All => {
                let list_order = self.list_names();
                let mut sorted: Vec<&Item> = self.items.iter().collect();
                sorted.sort_by(|a, b| {
                    let a_pos = list_order
                        .iter()
                        .position(|n| *n == a.list)
                        .unwrap_or(usize::MAX);
                    let b_pos = list_order
                        .iter()
                        .position(|n| *n == b.list)
                        .unwrap_or(usize::MAX);
                    a_pos.cmp(&b_pos).then(
                        a.order
                            .partial_cmp(&b.order)
                            .unwrap_or(std::cmp::Ordering::Equal),
                    )
                });
                sorted
            }
        }
    }

    pub fn selected_item(&self) -> Option<&Item> {
        let items = self.current_items();
        self.task_list_state
            .selected()
            .and_then(|i| items.get(i).copied())
    }

    pub fn next_queue(&mut self) {
        let len = self.sidebar_entries.len();
        self.active_sidebar_index = (self.active_sidebar_index + 1) % len;
        self.select_first_task();
    }

    pub fn prev_queue(&mut self) {
        let len = self.sidebar_entries.len();
        self.active_sidebar_index = (self.active_sidebar_index + len - 1) % len;
        self.select_first_task();
    }

    pub fn select_queue_by_index(&mut self, index: usize) {
        if let Some(sidebar_idx) = (index < self.sidebar_entries.len()).then_some(index) {
            self.active_sidebar_index = sidebar_idx;
            self.select_first_task();
        }
    }

    pub fn jump_to_list(&mut self, list_name: &str) {
        if let Some(idx) = self
            .sidebar_entries
            .iter()
            .position(|e| matches!(e, SidebarEntry::List(n) if n == list_name))
        {
            self.active_sidebar_index = idx;
            self.select_first_task();
        }
    }

    pub fn select_next_task(&mut self) {
        let count = self.current_items().len();
        if count == 0 {
            self.next_queue();
            return;
        }
        let current = self.task_list_state.selected().unwrap_or(0);
        if current + 1 < count {
            self.task_list_state.select(Some(current + 1));
            self.detail_scroll = 0;
        }
        // At bottom — stop, don't wrap or cross
    }

    pub fn select_prev_task(&mut self) {
        let count = self.current_items().len();
        if count == 0 {
            self.prev_queue();
            return;
        }
        let current = self.task_list_state.selected().unwrap_or(0);
        if current > 0 {
            self.task_list_state.select(Some(current - 1));
            self.detail_scroll = 0;
        }
        // At top — stop, don't wrap or cross
    }

    pub fn select_first_task_absolute(&mut self) {
        let count = self.current_items().len();
        if count > 0 {
            self.task_list_state.select(Some(0));
            self.detail_scroll = 0;
        }
    }

    pub fn select_last_task(&mut self) {
        let count = self.current_items().len();
        if count > 0 {
            self.task_list_state.select(Some(count - 1));
            self.detail_scroll = 0;
        }
    }

    pub fn visual_selection_range(&self) -> Option<(usize, usize)> {
        if let Mode::Visual { anchor } = &self.mode {
            let cursor = if self.focused_panel == FocusedPanel::Sidebar {
                self.active_sidebar_index
            } else {
                self.task_list_state.selected().unwrap_or(0)
            };
            let start = (*anchor).min(cursor);
            let end = (*anchor).max(cursor);
            Some((start, end))
        } else {
            None
        }
    }

    pub fn visual_selected_task_ids(&self) -> Vec<String> {
        if let Some((start, end)) = self.visual_selection_range() {
            let items = self.current_items();
            (start..=end)
                .filter_map(|i| items.get(i).map(|it| it.ext_id.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn list_names(&self) -> Vec<&str> {
        self.sidebar_entries
            .iter()
            .filter_map(|e| match e {
                SidebarEntry::List(n) => Some(n.as_str()),
                SidebarEntry::All => None,
            })
            .collect()
    }

    pub fn next_list_for(&self, list: &str) -> Option<String> {
        let names = self.list_names();
        let pos = names.iter().position(|n| *n == list)?;
        names.get(pos + 1).map(|s| s.to_string())
    }

    pub fn prev_list_for(&self, list: &str) -> Option<String> {
        let names = self.list_names();
        let pos = names.iter().position(|n| *n == list)?;
        (pos > 0).then(|| names[pos - 1].to_string())
    }

    /// Swap the current sidebar entry with the one below it and persist.
    pub fn swap_list_down(&mut self) {
        let len = self.sidebar_entries.len();
        let idx = self.active_sidebar_index;
        if idx + 1 >= len
            || matches!(self.sidebar_entries[idx], SidebarEntry::All)
            || matches!(self.sidebar_entries[idx + 1], SidebarEntry::All)
        {
            return;
        }
        self.sidebar_entries.swap(idx, idx + 1);
        self.active_sidebar_index = idx + 1;
        self.persist_list_order();
    }

    /// Swap the current sidebar entry with the one above it and persist.
    pub fn swap_list_up(&mut self) {
        let idx = self.active_sidebar_index;
        if idx == 0 || matches!(self.sidebar_entries[idx], SidebarEntry::All) {
            return;
        }
        self.sidebar_entries.swap(idx, idx - 1);
        self.active_sidebar_index = idx - 1;
        self.persist_list_order();
    }

    pub fn swap_list_block_down(&mut self) {
        self.swap_list_block(true);
    }
    pub fn swap_list_block_up(&mut self) {
        self.swap_list_block(false);
    }

    fn swap_list_block(&mut self, down: bool) {
        let (start, end) = self.sidebar_selection_range();
        if down {
            let len = self.sidebar_entries.len();
            if end + 1 >= len || matches!(self.sidebar_entries[end + 1], SidebarEntry::All) {
                return;
            }
            let item = self.sidebar_entries.remove(end + 1);
            self.sidebar_entries.insert(start, item);
            self.active_sidebar_index = end + 1;
            if let Mode::Visual { anchor } = &mut self.mode {
                *anchor = start + 1;
            }
        } else {
            if start == 0 {
                return;
            }
            let item = self.sidebar_entries.remove(start - 1);
            self.sidebar_entries.insert(end, item);
            self.active_sidebar_index = end - 1;
            if let Mode::Visual { anchor } = &mut self.mode {
                *anchor = start - 1;
            }
        }
        self.persist_list_order();
    }

    fn sidebar_selection_range(&self) -> (usize, usize) {
        if let Mode::Visual { anchor } = &self.mode {
            let cursor = self.active_sidebar_index;
            ((*anchor).min(cursor), (*anchor).max(cursor))
        } else {
            (self.active_sidebar_index, self.active_sidebar_index)
        }
    }

    fn persist_list_order(&mut self) {
        let lists: Vec<ListDef> = self
            .list_names()
            .iter()
            .enumerate()
            .map(|(i, name)| ListDef {
                name: name.to_string(),
                display: name.to_string(),
                order: i as f64,
            })
            .collect();
        let _ = self.adapter.set_lists(&lists);
    }

    pub fn update_search_results(&mut self) {
        let Mode::Search {
            query,
            results,
            list_state,
        } = &mut self.mode
        else {
            return;
        };
        *results = self
            .items
            .iter()
            .filter(|i| i.matches_query(query))
            .map(|i| (i.ext_id.clone(), i.list.clone()))
            .collect();
        if results.is_empty() {
            list_state.select(None);
        } else {
            list_state.select(Some(0));
        }
    }

    pub fn select_search_result(&mut self) {
        let Mode::Search {
            results,
            list_state,
            ..
        } = &self.mode
        else {
            return;
        };
        let Some(idx) = list_state.selected() else {
            return;
        };
        let Some((ext_id, list_name)) = results.get(idx).cloned() else {
            return;
        };
        self.jump_to_list(&list_name);
        let item_index = self
            .items
            .iter()
            .filter(|i| i.list == list_name)
            .position(|i| i.ext_id == ext_id)
            .unwrap_or(0);
        self.task_list_state.select(Some(item_index));
        self.detail_scroll = 0;
        self.mode = Mode::Normal;
    }

    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = Some((message.into(), Instant::now()));
    }

    pub fn active_status_message(&self) -> Option<&str> {
        self.status_message.as_ref().and_then(|(msg, when)| {
            if when.elapsed().as_secs() < STATUS_MESSAGE_TTL_SECS {
                Some(msg.as_str())
            } else {
                None
            }
        })
    }

    fn select_first_task(&mut self) {
        if self.current_items().is_empty() {
            self.task_list_state.select(None);
        } else {
            self.task_list_state.select(Some(0));
        }
        self.detail_scroll = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_filter_display() {
        assert_eq!(ListFilter::Single("now".into()).to_string(), "now");
        assert_eq!(ListFilter::All.to_string(), "all");
    }

    #[test]
    fn default_sidebar_has_all_at_end() {
        let entries = default_sidebar_entries();
        assert!(matches!(entries.last(), Some(SidebarEntry::All)));
        assert!(entries.len() >= 6);
    }
}
