use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render(frame: &mut Frame, name: &str) {
    let area = centered_rect(50, 7, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Add Sublist ")
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

    let name_line = Line::from(vec![
        Span::styled("Name: ", Style::default().fg(Color::Yellow)),
        Span::raw(format!("{name}\u{2588}")),
    ]);
    frame.render_widget(Paragraph::new(name_line), rows[0]);

    let hint = Line::from(vec![Span::styled(
        "Letters/digits/-/_ only",
        Style::default().fg(Color::Indexed(245)),
    )]);
    frame.render_widget(Paragraph::new(hint), rows[2]);

    let help = Line::from(vec![
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(":create  "),
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
