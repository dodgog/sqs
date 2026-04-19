use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::Line,
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

    let lines: Vec<Line> = item
        .body
        .lines()
        .map(|l| Line::from(l.to_string()))
        .collect();

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));

    frame.render_widget(paragraph, area);
}
