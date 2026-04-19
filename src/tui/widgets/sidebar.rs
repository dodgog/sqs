use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use super::panel_border_style;
use crate::tui::app_state::{FocusedPanel, SidebarEntry, TuiApp};

pub fn render(frame: &mut Frame, area: Rect, app: &TuiApp, focused: bool) {
    let counts = app.list_counts();
    let visual_range = if app.focused_panel == FocusedPanel::Sidebar {
        app.visual_selection_range()
    } else {
        None
    };

    let mut items: Vec<ListItem> = Vec::new();
    // Track sidebar entry index separately from display row index
    // (because All inserts an extra separator row)
    for (sidebar_idx, entry) in app.sidebar_entries().iter().enumerate() {
        match entry {
            SidebarEntry::List(name) => {
                let is_active = sidebar_idx == app.active_sidebar_index;
                let is_selected = visual_range
                    .is_some_and(|(start, end)| sidebar_idx >= start && sidebar_idx <= end);
                let mut item = list_item(name, counts.get(name), is_active);
                if is_selected {
                    item = item.style(Style::default().bg(Color::DarkGray));
                }
                items.push(item);
            }
            SidebarEntry::All => {
                items.push(ListItem::new(Line::from(Span::styled(
                    "  ──────────",
                    Style::default().fg(Color::Indexed(245)),
                ))));
                let is_active = sidebar_idx == app.active_sidebar_index;
                let is_selected = visual_range
                    .is_some_and(|(start, end)| sidebar_idx >= start && sidebar_idx <= end);
                let mut item = list_item("all", counts.total, is_active);
                if is_selected {
                    item = item.style(Style::default().bg(Color::DarkGray));
                }
                items.push(item);
            }
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(panel_border_style(focused));
    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn list_item(label: &str, count: usize, is_active: bool) -> ListItem<'static> {
    let marker = if is_active { ">" } else { " " };
    let line = Line::from(vec![
        Span::raw(format!("{marker} ")),
        Span::styled(format!("{:<6}", label), Style::default().fg(Color::Magenta)),
        Span::styled(format!("{count:>3}"), Style::default().fg(Color::Yellow)),
    ]);
    ListItem::new(line)
}
