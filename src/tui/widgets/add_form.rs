use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render(frame: &mut Frame, title: &str, list: &str) {
    let area = centered_rect(50, 7, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Add Item ")
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let title_line = Line::from(vec![
        Span::styled("Title: ", Style::default().fg(Color::Yellow)),
        Span::raw(format!("{title}\u{2588}")),
    ]);
    frame.render_widget(Paragraph::new(title_line), rows[0]);

    let list_line = Line::from(vec![
        Span::styled("List: ", Style::default().fg(Color::Yellow)),
        Span::styled(list.to_string(), Style::default().fg(Color::Magenta)),
        Span::raw("  (Tab to change)"),
    ]);
    frame.render_widget(Paragraph::new(list_line), rows[1]);

    let help = Line::from(vec![
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(":create  "),
        Span::styled("Tab", Style::default().fg(Color::Yellow)),
        Span::raw(":list  "),
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
