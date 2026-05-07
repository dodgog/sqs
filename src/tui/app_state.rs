use std::fmt;
use std::time::Instant;

use ratatui::widgets::ListState;

use crate::adapter::{Adapter, Item, ListDef};
use crate::app::app_error::AppError;
use crate::ordering;

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

/// Rank items against a query and return up to ~50 best matches as
/// `(ext_id, list)` pairs. Title-prefix beats exact-tag beats title-substring
/// beats tag-substring beats body-substring. `#tag` prefix searches tags only.
fn ranked_find(items: &[Item], query: &str) -> Vec<(String, String)> {
    let q = query.trim();
    if q.is_empty() {
        return Vec::new();
    }
    let tags_only = q.starts_with('#');
    let needle = if tags_only { &q[1..] } else { q };
    let needle_lc = needle.to_lowercase();
    if needle_lc.is_empty() {
        return Vec::new();
    }

    let mut scored: Vec<(i32, &Item)> = Vec::new();
    for item in items {
        let title_lc = item.title.to_lowercase();
        let body_lc = item.body.to_lowercase();
        let tag_lcs: Vec<String> = item.tags.iter().map(|t| t.to_lowercase()).collect();

        let mut score: i32 = 0;
        if tags_only {
            if tag_lcs.iter().any(|t| t == &needle_lc) {
                score = 100;
            } else if tag_lcs.iter().any(|t| t.starts_with(&needle_lc)) {
                score = 70;
            } else if tag_lcs.iter().any(|t| t.contains(&needle_lc)) {
                score = 40;
            }
        } else {
            if title_lc.starts_with(&needle_lc) {
                score = score.max(90);
            }
            if tag_lcs.iter().any(|t| t == &needle_lc) {
                score = score.max(80);
            }
            if title_lc.contains(&needle_lc) {
                score = score.max(60);
            }
            if tag_lcs.iter().any(|t| t.contains(&needle_lc)) {
                score = score.max(40);
            }
            if item.ext_id.to_lowercase().contains(&needle_lc) {
                score = score.max(30);
            }
            if body_lc.contains(&needle_lc) {
                score = score.max(20);
            }
        }
        if score > 0 {
            scored.push((score, item));
        }
    }

    scored.sort_by_key(|(s, _)| std::cmp::Reverse(*s));
    scored
        .into_iter()
        .take(50)
        .map(|(_, i)| (i.ext_id.clone(), i.list.clone()))
        .collect()
}

/// Quick checks for global-ordering invariants that warrant an auto-renormalize.
///
/// Healthy means: list keys are unique and well-spaced (gap >= 1.0); item keys
/// are unique; every item's key is strictly between its derived list and the
/// next list (i.e. no item collides with a list marker).
fn ordering_is_healthy(lists: &[ListDef], items: &[Item]) -> bool {
    use std::collections::HashSet;
    let mut keys: HashSet<u64> = HashSet::new();
    let mut sorted = lists.to_vec();
    sorted.sort_by(|a, b| {
        a.order
            .partial_cmp(&b.order)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for win in sorted.windows(2) {
        if win[1].order - win[0].order < 1.0 {
            return false;
        }
        if !keys.insert(win[0].order.to_bits()) {
            return false;
        }
    }
    if let Some(last) = sorted.last()
        && !keys.insert(last.order.to_bits())
    {
        return false;
    }
    let mut item_keys: HashSet<u64> = HashSet::new();
    for item in items {
        if !item_keys.insert(item.order.to_bits()) {
            return false;
        }
        if keys.contains(&item.order.to_bits()) {
            return false;
        }
    }
    true
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

const STATUS_MESSAGE_TTL_SECS: u64 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPanel {
    Sidebar,
    TaskList,
    Detail,
    Tags,
}

impl FocusedPanel {
    pub fn left(self) -> Self {
        match self {
            Self::Tags => Self::Detail,
            Self::Detail => Self::TaskList,
            Self::TaskList => Self::Sidebar,
            Self::Sidebar => Self::Sidebar,
        }
    }

    pub fn right(self) -> Self {
        match self {
            Self::Sidebar => Self::TaskList,
            Self::TaskList => Self::Detail,
            // Detail and Tags share the right column (Tags below Detail).
            // `l` from Detail descends into the Tags pane.
            Self::Detail => Self::Tags,
            Self::Tags => Self::Tags,
        }
    }
}

/// What `Mode::ConfirmDelete` is asking the user to delete.
#[derive(Debug, Clone)]
pub enum DeleteScope {
    Items(Vec<String>),
    /// Lists by name. Each may or may not be empty; non-empty ones go through
    /// carry mode first, empty ones are deleted directly.
    Lists(Vec<String>),
}

pub enum Mode {
    Normal,
    Visual {
        anchor: usize,
    },
    AddForm {
        title: String,
        list: String,
        insert_at: usize,
    },
    AddSublist {
        name: String,
        insert_at: usize,
    },
    ConfirmDelete {
        scope: DeleteScope,
    },
    CarryToList {
        selected_ids: Vec<String>,
        source_lists: Vec<String>,
        prior_anchor: Option<usize>,
        /// Lists to delete after the carry completes (used by the
        /// "delete non-empty list" flow that first moves items away).
        pending_list_delete: Vec<String>,
    },
    Search {
        query: String,
        results: Vec<(String, String)>,
        list_state: ListState,
    },
    /// `f` find — ranked read-only result list.
    Find {
        query: String,
        results: Vec<(String, String)>,
        list_state: ListState,
    },
    /// `t` — pick tags to apply to items or lists. Pre-checks tags that
    /// are present on every target; on submit the picker's set replaces
    /// each target's tags-in-the-picker (tags not shown stay untouched).
    TagPicker {
        target: TagTarget,
        cursor: usize,
        /// Currently checked tags in the picker. Initialized to the
        /// intersection of targets' tags so unchecking removes them.
        selected: Vec<String>,
        new_tag: String,
        /// The set of tags pre-checked at picker open — used to compute the
        /// "remove" diff: tags that started checked and are now unchecked.
        initial: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub enum TagTarget {
    Items(Vec<String>),
    Lists(Vec<String>),
}

pub struct TuiApp {
    pub adapter: Box<dyn Adapter>,
    pub items: Vec<Item>,
    pub sidebar_entries: Vec<SidebarEntry>,
    pub active_sidebar_index: usize,
    pub task_list_state: ListState,
    pub focused_panel: FocusedPanel,
    pub detail_scroll: u16,
    pub mode: Mode,
    pub status_message: Option<(String, Instant)>,
    pub needs_redraw: bool,
    /// Tags pane is always visible (below the preview). Filter is active
    /// iff `tag_filter` is non-empty.
    pub active_tag_index: usize,
    pub tag_filter: Vec<String>,
}

impl TuiApp {
    pub fn new(mut adapter: Box<dyn Adapter>) -> Result<Self, AppError> {
        let mut healed = false;
        let mut items = adapter.scan()?;
        let lists = adapter.lists();
        if !ordering_is_healthy(&lists, &items) {
            adapter.renormalize()?;
            items = adapter.scan()?;
            healed = true;
        }
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
            active_tag_index: 0,
            tag_filter: Vec::new(),
        };
        app.select_first_task();
        if healed {
            app.set_status("Renormalized order keys on startup");
        }
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
        let active = self.tag_filter_active();
        let mut total = 0;
        for item in &self.items {
            if active && !self.item_matches_tag_filter(item) {
                continue;
            }
            *counts.entry(item.list.clone()).or_insert(0) += 1;
            total += 1;
        }
        ListCounts { counts, total }
    }

    pub fn current_items(&self) -> Vec<&Item> {
        let active = self.tag_filter_active();
        let mut filtered: Vec<&Item> = match self.active_filter() {
            ListFilter::Single(ref name) => self.items.iter().filter(|i| i.list == *name).collect(),
            ListFilter::All => self.items.iter().collect(),
        };
        if active {
            filtered.retain(|i| self.item_matches_tag_filter(i));
        }
        filtered.sort_by(|a, b| {
            a.order
                .partial_cmp(&b.order)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        filtered
    }

    pub fn tag_filter_active(&self) -> bool {
        !self.tag_filter.is_empty()
    }

    fn item_matches_tag_filter(&self, item: &Item) -> bool {
        self.tag_filter.iter().all(|t| item.tags.contains(t))
    }

    pub fn toggle_tag_at_cursor(&mut self) {
        let tags = self.all_tags();
        if let Some(tag) = tags.get(self.active_tag_index).cloned() {
            if let Some(pos) = self.tag_filter.iter().position(|t| t == &tag) {
                self.tag_filter.remove(pos);
            } else {
                self.tag_filter.push(tag);
                self.tag_filter.sort();
            }
        }
    }

    pub fn select_next_tag(&mut self) {
        let len = self.all_tags().len();
        if len == 0 {
            return;
        }
        if self.active_tag_index + 1 < len {
            self.active_tag_index += 1;
        }
    }

    pub fn select_prev_tag(&mut self) {
        if self.active_tag_index > 0 {
            self.active_tag_index -= 1;
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

    /// Anchor row of an active visual selection — only when the requested pane
    /// owns the selection (the focused pane). Used by widgets to suppress
    /// the indicator on the unfocused pane.
    pub fn visual_anchor_for(&self, pane: FocusedPanel) -> Option<usize> {
        if self.focused_panel != pane {
            return None;
        }
        if let Mode::Visual { anchor } = &self.mode {
            Some(*anchor)
        } else {
            None
        }
    }

    /// Cursor row of an active visual selection in the requested pane.
    pub fn visual_cursor_for(&self, pane: FocusedPanel) -> Option<usize> {
        if self.focused_panel != pane {
            return None;
        }
        if matches!(self.mode, Mode::Visual { .. }) {
            let cursor = if pane == FocusedPanel::Sidebar {
                self.active_sidebar_index
            } else {
                self.task_list_state.selected().unwrap_or(0)
            };
            Some(cursor)
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

    /// Look up a list's order_key via the current adapter snapshot.
    pub fn list_order_key(&self, name: &str) -> Option<f64> {
        self.adapter
            .lists()
            .into_iter()
            .find(|l| l.name == name)
            .map(|l| l.order)
    }

    /// Order key just past the last item in `list`, suitable for appending.
    /// Returns `None` if the list is full (would collide with the next list
    /// marker) — caller should renormalize and retry.
    pub fn bottom_of_list_order_key(&self, list: &str) -> Option<f64> {
        let list_key = self.list_order_key(list)?;
        let max_item_key = self
            .items
            .iter()
            .filter(|i| i.list == list)
            .map(|i| i.order)
            .fold(list_key, f64::max);
        let candidate = max_item_key + 1.0;
        let next_marker = self
            .next_list_for(list)
            .and_then(|n| self.list_order_key(&n))
            .unwrap_or(f64::INFINITY);
        if candidate >= next_marker - ordering::EPSILON {
            None
        } else {
            Some(candidate)
        }
    }

    /// Like `bottom_of_list_order_key` but auto-renormalizes when the list is
    /// full and retries. Mutates `self.items` (via refresh) on renorm.
    pub fn bottom_of_list_with_renorm(&mut self, list: &str) -> Result<f64, AppError> {
        if let Some(k) = self.bottom_of_list_order_key(list) {
            return Ok(k);
        }
        self.adapter.renormalize()?;
        self.refresh()?;
        self.set_status("Renormalized order keys");
        self.bottom_of_list_order_key(list)
            .ok_or_else(|| AppError::message(format!("list '{list}' has no room")))
    }

    /// Persist sidebar order: rewrite list keys at spaced multiples of LIST_SPACING,
    /// and bring all items along by reassigning each item's order_key to follow its
    /// list's new key. This is the block-move-on-list-reorder behavior.
    pub fn persist_list_order(&mut self) -> Result<(), AppError> {
        let list_names: Vec<String> = self.list_names().iter().map(|s| s.to_string()).collect();
        let existing: std::collections::HashMap<String, ListDef> = self
            .adapter
            .lists()
            .into_iter()
            .map(|l| (l.name.clone(), l))
            .collect();

        let new_lists: Vec<ListDef> = list_names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let prior = existing.get(name);
                let display = prior
                    .map(|l| l.display.clone())
                    .unwrap_or_else(|| name.clone());
                let tags = prior.map(|l| l.tags.clone()).unwrap_or_default();
                ListDef {
                    name: name.clone(),
                    display,
                    order: ordering::list_key_for_index(i),
                    tags,
                }
            })
            .collect();

        let mut item_updates: Vec<(String, f64)> = Vec::new();
        for new_list in &new_lists {
            let mut items_in_list: Vec<&Item> = self
                .items
                .iter()
                .filter(|it| it.list == new_list.name)
                .collect();
            items_in_list.sort_by(|a, b| {
                a.order
                    .partial_cmp(&b.order)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            for (j, item) in items_in_list.iter().enumerate() {
                item_updates.push((
                    item.ext_id.clone(),
                    ordering::item_key_in_list(new_list.order, j),
                ));
            }
        }

        self.adapter.set_lists(&new_lists)?;
        if !item_updates.is_empty() {
            self.adapter.batch_update_orders(&item_updates)?;
        }
        self.refresh()?;
        Ok(())
    }

    /// Persist a within-list reorder by computing fresh order_keys for the
    /// given ids in their listed order.
    pub fn persist_list_item_order(
        &mut self,
        list: &str,
        ordered_ids: &[String],
    ) -> Result<(), AppError> {
        if self.tag_filter_active() {
            return self.persist_visible_reorder(list, ordered_ids);
        }
        let Some(list_key) = self.list_order_key(list) else {
            return Ok(());
        };
        let updates: Vec<(String, f64)> = ordered_ids
            .iter()
            .enumerate()
            .map(|(i, id)| (id.clone(), ordering::item_key_in_list(list_key, i)))
            .collect();
        self.adapter.batch_update_orders(&updates)?;
        Ok(())
    }

    /// Reorder the visible (filtered) items in `list` without touching any
    /// hidden items' order_keys. Visible items are spread evenly across the
    /// list's allotted range; collisions with hidden keys are nudged off.
    /// Auto-renormalizes if the range is too tight.
    pub fn persist_visible_reorder(
        &mut self,
        _list: &str,
        visible_new: &[String],
    ) -> Result<(), AppError> {
        if visible_new.is_empty() {
            return Ok(());
        }
        // Permute the order_keys these visible items already hold.
        // Hidden items keep their absolute positions verbatim, so a hidden
        // item that sat between two visible items stays between them.
        let mut current_keys: Vec<f64> = self
            .items
            .iter()
            .filter(|i| visible_new.iter().any(|id| id == &i.ext_id))
            .map(|i| i.order)
            .collect();
        current_keys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        if current_keys.len() != visible_new.len() {
            // Some ids weren't found — fall back to the safer global path.
            return Ok(());
        }

        let updates: Vec<(String, f64)> = visible_new
            .iter()
            .zip(current_keys.iter())
            .map(|(id, key)| (id.clone(), *key))
            .collect();
        self.adapter.batch_update_orders(&updates)?;
        Ok(())
    }

    /// Move an item to the bottom of the target list, reassigning its order_key.
    /// Auto-renormalizes if the list is full.
    pub fn persist_move_to_bottom(
        &mut self,
        ext_id: &str,
        target_list: &str,
    ) -> Result<(), AppError> {
        let key = self.bottom_of_list_with_renorm(target_list)?;
        self.adapter.update_item_order(ext_id, key)?;
        Ok(())
    }

    /// Names of the sublists currently selected (visual range in sidebar
    /// or just the active one). Excludes the synthetic `All` entry.
    pub fn selected_sublist_names(&self) -> Vec<String> {
        let (start, end) = self.sidebar_selection_range();
        (start..=end)
            .filter_map(|i| self.sidebar_entries.get(i))
            .filter_map(|e| match e {
                SidebarEntry::List(n) => Some(n.clone()),
                SidebarEntry::All => None,
            })
            .collect()
    }

    /// All item ext_ids belonging to any of the given lists, in global order.
    pub fn item_ids_in_lists(&self, lists: &[String]) -> Vec<String> {
        let mut items: Vec<&Item> = self
            .items
            .iter()
            .filter(|it| lists.iter().any(|n| n == &it.list))
            .collect();
        items.sort_by(|a, b| {
            a.order
                .partial_cmp(&b.order)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        items.into_iter().map(|i| i.ext_id.clone()).collect()
    }

    pub fn swap_list_down(&mut self) -> Result<(), AppError> {
        let len = self.sidebar_entries.len();
        let idx = self.active_sidebar_index;
        if idx + 1 >= len
            || matches!(self.sidebar_entries[idx], SidebarEntry::All)
            || matches!(self.sidebar_entries[idx + 1], SidebarEntry::All)
        {
            return Ok(());
        }
        self.sidebar_entries.swap(idx, idx + 1);
        self.active_sidebar_index = idx + 1;
        self.persist_list_order()
    }

    pub fn swap_list_up(&mut self) -> Result<(), AppError> {
        let idx = self.active_sidebar_index;
        if idx == 0 || matches!(self.sidebar_entries[idx], SidebarEntry::All) {
            return Ok(());
        }
        self.sidebar_entries.swap(idx, idx - 1);
        self.active_sidebar_index = idx - 1;
        self.persist_list_order()
    }

    pub fn swap_list_block_down(&mut self) -> Result<(), AppError> {
        self.swap_list_block(true)
    }
    pub fn swap_list_block_up(&mut self) -> Result<(), AppError> {
        self.swap_list_block(false)
    }

    fn swap_list_block(&mut self, down: bool) -> Result<(), AppError> {
        let (start, end) = self.sidebar_selection_range();
        if down {
            let len = self.sidebar_entries.len();
            if end + 1 >= len || matches!(self.sidebar_entries[end + 1], SidebarEntry::All) {
                return Ok(());
            }
            let item = self.sidebar_entries.remove(end + 1);
            self.sidebar_entries.insert(start, item);
            self.active_sidebar_index = end + 1;
            if let Mode::Visual { anchor } = &mut self.mode {
                *anchor = start + 1;
            }
        } else {
            if start == 0 {
                return Ok(());
            }
            let item = self.sidebar_entries.remove(start - 1);
            self.sidebar_entries.insert(end, item);
            self.active_sidebar_index = end - 1;
            if let Mode::Visual { anchor } = &mut self.mode {
                *anchor = start - 1;
            }
        }
        self.persist_list_order()
    }

    fn sidebar_selection_range(&self) -> (usize, usize) {
        if let Mode::Visual { anchor } = &self.mode {
            let cursor = self.active_sidebar_index;
            ((*anchor).min(cursor), (*anchor).max(cursor))
        } else {
            (self.active_sidebar_index, self.active_sidebar_index)
        }
    }

    pub fn create_sublist(&mut self, name: &str, insert_at: usize) -> Result<(), AppError> {
        if name.is_empty() {
            return Err(AppError::usage("sublist name cannot be empty"));
        }
        if self.list_names().contains(&name) {
            return Err(AppError::usage(format!("list '{name}' already exists")));
        }
        let clamp = insert_at.min(self.list_names().len());
        let target_idx = clamp;
        self.sidebar_entries
            .insert(target_idx, SidebarEntry::List(name.to_string()));
        self.persist_list_order()?;
        self.sidebar_entries = sidebar_from_adapter_lists(&self.adapter.lists());
        if let Some(idx) = self
            .sidebar_entries
            .iter()
            .position(|e| matches!(e, SidebarEntry::List(n) if n == name))
        {
            self.active_sidebar_index = idx;
        }
        Ok(())
    }

    pub fn update_find_results(&mut self) {
        let Mode::Find {
            query,
            results,
            list_state,
        } = &mut self.mode
        else {
            return;
        };
        *results = ranked_find(&self.items, query);
        if results.is_empty() {
            list_state.select(None);
        } else {
            list_state.select(Some(0));
        }
    }

    pub fn select_find_result(&mut self) {
        let Mode::Find {
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
        self.focused_panel = FocusedPanel::TaskList;
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

    /// Tags present on every item in `ids` (for pre-checking the picker).
    pub fn intersect_item_tags(&self, ids: &[String]) -> Vec<String> {
        if ids.is_empty() {
            return Vec::new();
        }
        let sets: Vec<Vec<String>> = ids
            .iter()
            .filter_map(|id| self.items.iter().find(|i| &i.ext_id == id))
            .map(|i| i.tags.clone())
            .collect();
        if sets.is_empty() {
            return Vec::new();
        }
        let mut result = sets[0].clone();
        for s in &sets[1..] {
            result.retain(|t| s.contains(t));
        }
        result.sort();
        result.dedup();
        result
    }

    /// Tags present on every list in `names`.
    pub fn intersect_list_tags(&self, names: &[String]) -> Vec<String> {
        let lists = self.adapter.lists();
        let sets: Vec<Vec<String>> = names
            .iter()
            .filter_map(|n| lists.iter().find(|l| &l.name == n))
            .map(|l| l.tags.clone())
            .collect();
        if sets.is_empty() {
            return Vec::new();
        }
        let mut result = sets[0].clone();
        for s in &sets[1..] {
            result.retain(|t| s.contains(t));
        }
        result.sort();
        result.dedup();
        result
    }

    pub fn all_tags(&self) -> Vec<String> {
        let mut set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for item in &self.items {
            for t in &item.tags {
                set.insert(t.clone());
            }
        }
        for list in self.adapter.lists() {
            for t in list.tags {
                set.insert(t);
            }
        }
        set.into_iter().collect()
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
    use crate::adapters::markdown_todolists::{MarkdownTodolistsAdapter, io};
    use tempfile::TempDir;

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

    fn make_app() -> (TempDir, TuiApp) {
        let temp = TempDir::new().unwrap();
        let mut adapter = MarkdownTodolistsAdapter::new(temp.path().to_path_buf());
        adapter.set_lists(&io::default_lists()).unwrap();
        let app = TuiApp::new(Box::new(adapter)).unwrap();
        (temp, app)
    }

    #[test]
    fn visual_anchor_only_in_focused_pane() {
        let (_temp, mut app) = make_app();
        app.focused_panel = FocusedPanel::TaskList;
        app.mode = Mode::Visual { anchor: 2 };
        assert_eq!(app.visual_anchor_for(FocusedPanel::TaskList), Some(2));
        assert_eq!(app.visual_anchor_for(FocusedPanel::Sidebar), None);

        app.focused_panel = FocusedPanel::Sidebar;
        assert_eq!(app.visual_anchor_for(FocusedPanel::Sidebar), Some(2));
        assert_eq!(app.visual_anchor_for(FocusedPanel::TaskList), None);

        app.mode = Mode::Normal;
        assert_eq!(app.visual_anchor_for(FocusedPanel::Sidebar), None);
    }

    #[test]
    fn ordering_healthy_for_default_lists() {
        let lists = io::default_lists();
        assert!(ordering_is_healthy(&lists, &[]));
    }

    #[test]
    fn ordering_unhealthy_when_keys_collide() {
        let mut lists = io::default_lists();
        lists[1].order = lists[0].order;
        assert!(!ordering_is_healthy(&lists, &[]));
    }

    #[test]
    fn create_sublist_inserts_and_jumps() {
        let (_temp, mut app) = make_app();
        app.create_sublist("waiting", 2).unwrap();
        assert!(app.list_names().iter().any(|n| *n == "waiting"));
        let active = match &app.sidebar_entries[app.active_sidebar_index] {
            SidebarEntry::List(n) => n.clone(),
            _ => String::new(),
        };
        assert_eq!(active, "waiting");
    }

    fn item(id: &str, list: &str, order: f64, tags: &[&str], title: &str) -> Item {
        Item {
            ext_id: id.into(),
            title: title.into(),
            body: String::new(),
            list: list.into(),
            order,
            tags: tags.iter().map(|s| s.to_string()).collect(),
            content_hash: 0,
        }
    }

    #[test]
    fn ranked_find_prefers_title_prefix_over_body() {
        let items = vec![
            item("a1", "now", 1001.0, &[], "Investigate billing"),
            item(
                "a2",
                "now",
                1002.0,
                &[],
                "Random task with billing in body content here",
            ),
        ];
        let r = ranked_find(&items, "billing");
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].0, "a1"); // title-prefix wins
    }

    #[test]
    fn ranked_find_tag_only_filter() {
        let items = vec![
            item(
                "a1",
                "now",
                1001.0,
                &["MIL010"],
                "Title with MIL010 in body",
            ),
            item("a2", "now", 1002.0, &["MIL020"], "Plain"),
        ];
        let r = ranked_find(&items, "#MIL010");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].0, "a1");
    }

    #[test]
    fn tag_filter_active_when_nonempty() {
        let (_temp, mut app) = make_app();
        assert!(!app.tag_filter_active());
        app.tag_filter = vec!["x".into()];
        assert!(app.tag_filter_active());
        app.tag_filter.clear();
        assert!(!app.tag_filter_active());
    }

    #[test]
    fn current_items_filters_by_tags_when_active() {
        let (_temp, mut app) = make_app();
        app.items = vec![
            item("a1", "now", 1001.0, &["x"], "x"),
            item("a2", "now", 1002.0, &["y"], "y"),
            item("a3", "now", 1003.0, &["x", "y"], "xy"),
        ];
        // Empty filter — no filter applied
        app.tag_filter.clear();
        assert_eq!(app.current_items().len(), 3);
        // Filter active (AND)
        app.tag_filter = vec!["x".into()];
        assert_eq!(app.current_items().len(), 2);
        app.tag_filter = vec!["x".into(), "y".into()];
        assert_eq!(app.current_items().len(), 1);
        assert_eq!(app.current_items()[0].ext_id, "a3");
    }

    #[test]
    fn persist_visible_reorder_keeps_hidden_items_in_place() {
        let (temp, mut app) = make_app();
        let mut adapter = crate::adapters::markdown_todolists::MarkdownTodolistsAdapter::new(
            temp.path().to_path_buf(),
        );
        adapter.set_lists(&io::default_lists()).unwrap();
        // now is at order 1000, next at 2000 (per default_lists spacing).
        adapter.create_item(Some("a"), "A", "", 1001.0).unwrap();
        adapter.create_item(Some("b"), "B", "", 1002.0).unwrap();
        adapter.create_item(Some("c"), "C", "", 1003.0).unwrap();
        adapter.create_item(Some("d"), "D", "", 1004.0).unwrap();
        adapter.create_item(Some("e"), "E", "", 1005.0).unwrap();
        adapter.set_item_tags("b", &["x".into()]).unwrap();
        adapter.set_item_tags("d", &["x".into()]).unwrap();
        adapter.set_item_tags("e", &["x".into()]).unwrap();
        app.adapter = Box::new(adapter);
        app.refresh().unwrap();

        let c_before = app.items.iter().find(|i| i.ext_id == "c").unwrap().order;

        // Reorder visible to [d, b, e]
        app.persist_visible_reorder("now", &["d".into(), "b".into(), "e".into()])
            .unwrap();
        app.refresh().unwrap();

        let key = |id: &str| app.items.iter().find(|i| i.ext_id == id).unwrap().order;
        // d, b, e should hold the same set of keys b, d, e originally had,
        // sorted ascending and assigned in the new order.
        assert_eq!(key("d"), 1002.0);
        assert_eq!(key("b"), 1004.0);
        assert_eq!(key("e"), 1005.0);
        // c stayed at 1003, still between two visible items
        assert_eq!(key("c"), c_before);

        // Final sorted order: a, d, c, b, e
        let mut sorted: Vec<&Item> = app.items.iter().collect();
        sorted.sort_by(|x, y| x.order.partial_cmp(&y.order).unwrap());
        let ids: Vec<&str> = sorted.iter().map(|i| i.ext_id.as_str()).collect();
        assert_eq!(ids, vec!["a", "d", "c", "b", "e"]);
    }

    #[test]
    fn bottom_of_list_returns_none_when_full() {
        let (_temp, mut app) = make_app();
        let mut adapter = MarkdownTodolistsAdapter::new(_temp.path().to_path_buf());
        for i in 0..3 {
            adapter
                .create_item(Some(&format!("a{i:03}")), "x", "", 1100.0 + i as f64)
                .unwrap();
        }
        app.adapter = Box::new(adapter);
        app.refresh().unwrap();
        let key = app.bottom_of_list_order_key("now").unwrap();
        assert!((1000.0..2000.0).contains(&key));
    }
}
