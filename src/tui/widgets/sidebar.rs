use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use super::{VisualMarkers, panel_border_style};
use crate::tui::app_state::{FocusedPanel, Mode, SidebarEntry, TuiApp};

pub fn render(frame: &mut Frame, area: Rect, app: &TuiApp, focused: bool) {
    let counts = app.list_counts();
    let markers = VisualMarkers::for_pane(
        Some(app.active_sidebar_index),
        app.visual_anchor_for(FocusedPanel::Sidebar),
        app.visual_cursor_for(FocusedPanel::Sidebar),
    );
    let (carry_active, source_lists): (bool, &[String]) = match &app.mode {
        Mode::CarryToList { source_lists, .. } => (true, source_lists.as_slice()),
        _ => (false, &[]),
    };

    let mut items: Vec<ListItem> = Vec::new();
    for (sidebar_idx, entry) in app.sidebar_entries().iter().enumerate() {
        match entry {
            SidebarEntry::List(name) => {
                let is_target = carry_active && sidebar_idx == app.active_sidebar_index;
                let is_source = carry_active && source_lists.iter().any(|s| s == name);
                items.push(list_item(
                    name,
                    counts.get(name),
                    sidebar_idx,
                    &markers,
                    is_target,
                    is_source,
                ));
            }
            SidebarEntry::All => {
                items.push(ListItem::new(Line::from(Span::styled(
                    "  ──────────",
                    Style::default().fg(Color::Indexed(245)),
                ))));
                let is_target = carry_active && sidebar_idx == app.active_sidebar_index;
                items.push(list_item(
                    "all",
                    counts.total,
                    sidebar_idx,
                    &markers,
                    is_target,
                    false,
                ));
            }
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(panel_border_style(focused));
    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn list_item(
    label: &str,
    count: usize,
    row: usize,
    markers: &VisualMarkers,
    carry_target: bool,
    carry_source: bool,
) -> ListItem<'static> {
    // Sidebar pane is narrow — paint carry markers in the second column
    // (the same column visual-anchor uses; carry and visual modes are
    // mutually exclusive). Target `*` wins over source `o` if both apply.
    let second_col = if carry_target {
        Span::styled(
            "*",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    } else if carry_source {
        Span::styled(
            "o",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw(markers.anchor_glyph(row).to_string())
    };
    let spans = vec![
        Span::raw(markers.cursor_glyph(row).to_string()),
        second_col,
        Span::raw(" "),
        Span::styled(format!("{:<6}", label), Style::default().fg(Color::Magenta)),
        Span::styled(format!("{count:>3}"), Style::default().fg(Color::Yellow)),
    ];
    let mut item = ListItem::new(Line::from(spans));
    if markers.should_highlight(row) {
        item = item.style(Style::default().bg(Color::DarkGray));
    }
    item
}
