use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use super::panel_border_style;
use crate::tui::app_state::TuiApp;

pub fn render(frame: &mut Frame, area: Rect, app: &TuiApp, focused: bool) {
    let tags = app.all_tags();
    let mut items: Vec<ListItem> = Vec::new();

    if tags.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "  (no tags)",
            Style::default().fg(Color::Indexed(245)),
        ))));
    } else {
        for (i, tag) in tags.iter().enumerate() {
            let selected = app.tag_filter.iter().any(|t| t == tag);
            let cursor = focused && i == app.active_tag_index;
            let marker = if cursor { ">" } else { " " };
            let checkbox = if selected { "[x]" } else { "[ ]" };
            let mut style = Style::default();
            if selected {
                style = style.fg(Color::Cyan).add_modifier(Modifier::BOLD);
            }
            let line = Line::from(vec![
                Span::raw(format!("{marker} ")),
                Span::raw(format!("{checkbox} ")),
                Span::styled(tag.clone(), style),
            ]);
            let mut item = ListItem::new(line);
            if cursor {
                item = item.style(Style::default().bg(Color::DarkGray));
            }
            items.push(item);
        }
    }

    let title = if app.tag_filter.is_empty() {
        " Tags ".to_string()
    } else {
        format!(" Tags │ {} active ", app.tag_filter.len())
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::styled(title, Style::default().fg(Color::White)))
        .border_style(panel_border_style(focused));
    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}
