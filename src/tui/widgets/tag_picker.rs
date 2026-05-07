use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

pub fn render(
    frame: &mut Frame,
    count: usize,
    kind_label: &str,
    all_tags: &[String],
    selected: &[String],
    new_tag: &str,
    cursor: usize,
) {
    // Tall enough that the help line never falls off the bottom.
    let area = centered_rect(60, 22, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Tag {count} {kind_label}(s) "))
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(2),
        ])
        .split(inner);

    let total = all_tags.len();
    let items: Vec<ListItem> = if total == 0 {
        vec![ListItem::new(Line::from(Span::styled(
            "(no existing tags — type a name below to add one)",
            Style::default().fg(Color::Indexed(245)),
        )))]
    } else {
        all_tags
            .iter()
            .enumerate()
            .map(|(i, tag)| {
                let is_selected = selected.iter().any(|t| t == tag);
                let is_cursor = i == cursor;
                let marker = if is_cursor { ">" } else { " " };
                let checkbox = if is_selected { "[x]" } else { "[ ]" };
                let mut style = Style::default();
                if is_selected {
                    style = style.fg(Color::Cyan).add_modifier(Modifier::BOLD);
                }
                let mut item = ListItem::new(Line::from(vec![
                    Span::raw(format!("{marker} ")),
                    Span::raw(format!("{checkbox} ")),
                    Span::styled(tag.clone(), style),
                ]));
                if is_cursor {
                    item = item.style(Style::default().bg(Color::DarkGray));
                }
                item
            })
            .collect()
    };
    frame.render_widget(List::new(items), rows[0]);

    let separator = Line::from(Span::styled(
        "─".repeat(60),
        Style::default().fg(Color::Indexed(245)),
    ));
    frame.render_widget(Paragraph::new(separator), rows[1]);

    let new_tag_line = Line::from(vec![
        Span::styled("New tag: ", Style::default().fg(Color::Yellow)),
        Span::raw(format!("{new_tag}\u{2588}")),
    ]);
    frame.render_widget(Paragraph::new(new_tag_line), rows[2]);

    let help = Line::from(vec![
        Span::styled("j/k", Style::default().fg(Color::Yellow)),
        Span::raw(":nav  "),
        Span::styled("Space", Style::default().fg(Color::Yellow)),
        Span::raw(":toggle  "),
        Span::styled("type", Style::default().fg(Color::Yellow)),
        Span::raw(":new  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(":apply  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(":cancel"),
    ]);
    frame.render_widget(Paragraph::new(help), rows[3]);
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
