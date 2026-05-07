use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::panel_border_style;
use crate::adapter::Item;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    item: Option<&Item>,
    scroll_offset: u16,
    focused: bool,
) {
    let border_style = panel_border_style(focused);

    let Some(item) = item else {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Detail ")
            .border_style(border_style);
        let empty = Paragraph::new("No item selected").block(block);
        frame.render_widget(empty, area);
        return;
    };

    let title = format!(" {} ", item.title);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::styled(title, Style::default().fg(Color::White)))
        .border_style(border_style);

    let mut lines: Vec<Line> = Vec::new();
    // Frontmatter header — same fields the YAML on disk holds.
    lines.push(field("id", &item.ext_id));
    lines.push(field("title", &item.title));
    lines.push(field("list", &item.list));
    lines.push(field("order", &format!("{}", item.order)));
    if !item.tags.is_empty() {
        lines.push(field("tags", &item.tags.join(" ")));
    } else {
        lines.push(field("tags", "(none)"));
    }
    lines.push(Line::from(Span::styled(
        "──────────",
        Style::default().fg(Color::Indexed(245)),
    )));
    lines.push(Line::from(""));
    for body_line in item.body.lines() {
        lines.push(Line::from(body_line.to_string()));
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));

    frame.render_widget(paragraph, area);
}

fn field(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(value.to_string()),
    ])
}
