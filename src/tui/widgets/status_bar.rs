use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::tui::app_state::{DeleteScope, Mode, TuiApp};

pub fn render(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let line = match &app.mode {
        Mode::Normal => normal_line(app, area.width),
        Mode::Visual { .. } => visual_line(),
        Mode::AddForm { .. }
        | Mode::AddSublist { .. }
        | Mode::Search { .. }
        | Mode::Find { .. }
        | Mode::TagPicker { .. } => return,
        Mode::ConfirmDelete { scope } => confirm_delete_line(scope),
        Mode::CarryToList { selected_ids, .. } => carry_to_list_line(selected_ids.len()),
    };

    let bar = Paragraph::new(line);
    frame.render_widget(bar, area);
}

fn normal_line(app: &TuiApp, area_width: u16) -> Line<'static> {
    if let Some(msg) = app.active_status_message() {
        return Line::from(vec![
            mode_badge("Normal"),
            Span::raw(" "),
            Span::styled(msg.to_string(), Style::default().fg(Color::Green)),
        ]);
    }

    let badge_label = if app.tag_filter_active() {
        "Normal*"
    } else {
        "Normal"
    };
    let mut spans = vec![
        mode_badge(badge_label),
        Span::raw(" "),
        hint("j/k"),
        Span::raw(":nav "),
        hint("J/K"),
        Span::raw(":reorder "),
        hint("v"),
        Span::raw(":select "),
        hint("a"),
        Span::raw(":add "),
        hint("m"),
        Span::raw(":move "),
        hint("f"),
        Span::raw(":find "),
        hint("/"),
        Span::raw(":search "),
        hint("t"),
        Span::raw(":tag "),
        hint("="),
        Span::raw(":renorm "),
        hint("e"),
        Span::raw(":edit "),
        hint("x"),
        Span::raw(":del "),
        hint("q"),
        Span::raw(":quit"),
    ];

    while spans.len() > 2 {
        let total_width: usize = spans.iter().map(|s| s.width()).sum();
        if total_width <= area_width as usize {
            break;
        }
        spans.pop();
        spans.pop();
    }

    Line::from(spans)
}

fn confirm_delete_line(scope: &DeleteScope) -> Line<'static> {
    let prompt = match scope {
        DeleteScope::Items(ids) if ids.len() == 1 => format!(" Delete item {}? ", ids[0]),
        DeleteScope::Items(ids) => format!(" Delete {} item(s)? ", ids.len()),
        DeleteScope::Lists(names) if names.len() == 1 => {
            format!(" Delete list '{}'? (will move items if any) ", names[0])
        }
        DeleteScope::Lists(names) => {
            format!(" Delete {} list(s)? (will move items if any) ", names.len())
        }
    };
    Line::from(vec![
        mode_badge("Delete"),
        Span::raw(prompt),
        hint("y"),
        Span::raw(":yes "),
        Span::raw("any other key:cancel"),
    ])
}

fn carry_to_list_line(count: usize) -> Line<'static> {
    Line::from(vec![
        mode_badge("Carry"),
        Span::raw(format!(" {count} item(s) │ ")),
        hint("j/k"),
        Span::raw(":nav "),
        hint("L/Enter"),
        Span::raw(":drop "),
        hint("Esc"),
        Span::raw(":cancel "),
        Span::styled("* target ", Style::default().fg(Color::Yellow)),
        Span::styled("o source", Style::default().fg(Color::Cyan)),
    ])
}

fn visual_line() -> Line<'static> {
    Line::from(vec![
        mode_badge("Visual"),
        Span::raw(" "),
        hint("j/k"),
        Span::raw(":extend "),
        hint("J/K"),
        Span::raw(":reorder "),
        hint("</>"),
        Span::raw(":send "),
        hint("H"),
        Span::raw(":carry "),
        hint("m"),
        Span::raw(":move "),
        hint("Esc"),
        Span::raw(":cancel"),
    ])
}

fn mode_badge(label: &str) -> Span<'static> {
    Span::styled(
        format!(" [{label}] "),
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
}

fn hint(key: &str) -> Span<'static> {
    Span::styled(key.to_string(), Style::default().fg(Color::Yellow))
}
