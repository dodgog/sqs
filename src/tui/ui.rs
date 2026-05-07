use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use super::{
    app_state::{FocusedPanel, Mode, TuiApp},
    widgets,
};

pub fn draw(frame: &mut Frame, app: &mut TuiApp) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    let main_area = outer[0];
    let status_area = outer[1];

    if matches!(app.mode, Mode::Search { .. }) {
        draw_search(frame, main_area, app);
    } else if matches!(app.mode, Mode::Find { .. }) {
        draw_find(frame, main_area, app);
    } else {
        draw_normal(frame, main_area, app);
    }

    widgets::status_bar::render(frame, status_area, app);

    if let Mode::AddForm { title, list, .. } = &app.mode {
        widgets::add_form::render(frame, title, list);
    }
    if let Mode::AddSublist { name, .. } = &app.mode {
        widgets::add_sublist::render(frame, name);
    }
    if let Mode::TagPicker {
        target,
        selected,
        new_tag,
        cursor,
        ..
    } = &app.mode
    {
        let tags = app.all_tags();
        let (count, kind_label) = match target {
            crate::tui::app_state::TagTarget::Items(ids) => (ids.len(), "item"),
            crate::tui::app_state::TagTarget::Lists(names) => (names.len(), "list"),
        };
        widgets::tag_picker::render(frame, count, kind_label, &tags, selected, new_tag, *cursor);
    }
}

fn draw_normal(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(14),
            Constraint::Min(20),
            Constraint::Percentage(35),
        ])
        .split(area);

    let sidebar_area = panels[0];
    let task_list_area = panels[1];
    let right_area = panels[2];

    // Split the right column: detail (top, ~70%) over tags (bottom, ~30%).
    let right_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Min(6)])
        .split(right_area);
    let detail_area = right_split[0];
    let tags_area = right_split[1];

    let focused = app.focused_panel;

    widgets::sidebar::render(frame, sidebar_area, app, focused == FocusedPanel::Sidebar);

    let filter = app.active_filter();
    let items = app.current_items();
    let selected_index = app.task_list_state.selected();
    let selected_item = selected_index.and_then(|i| items.get(i).copied()).cloned();
    let all_list_names: Vec<String> = app
        .sidebar_entries()
        .iter()
        .filter_map(|e| match e {
            crate::tui::app_state::SidebarEntry::List(n) => Some(n.clone()),
            _ => None,
        })
        .collect();
    widgets::task_list::render(
        frame,
        task_list_area,
        filter,
        &items,
        app,
        &all_list_names,
        focused == FocusedPanel::TaskList,
    );

    widgets::detail::render(
        frame,
        detail_area,
        selected_item.as_ref(),
        app.detail_scroll,
        focused == FocusedPanel::Detail,
    );

    widgets::tags::render(frame, tags_area, app, focused == FocusedPanel::Tags);
}

fn draw_search(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    let Mode::Search {
        query,
        results,
        list_state,
    } = &mut app.mode
    else {
        return;
    };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    let input = Paragraph::new(Line::from(vec![
        Span::styled("/ ", Style::default().fg(Color::Yellow)),
        Span::styled(
            format!("{}\u{2588}", query),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .title(format!(" Search ({} results) ", results.len())),
    );
    frame.render_widget(input, rows[0]);

    let items: Vec<ListItem> = results
        .iter()
        .filter_map(|(ext_id, list_name)| {
            let item = app.items.iter().find(|i| i.ext_id == *ext_id)?;
            let line = Line::from(vec![
                Span::styled(
                    format!("[{:<5}] ", list_name),
                    Style::default().fg(Color::Magenta),
                ),
                Span::styled(
                    format!("{:<6}", item.ext_id),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(&item.title),
            ]);
            Some(ListItem::new(line))
        })
        .collect();

    let highlight_style = Style::default().bg(Color::DarkGray);

    let list = List::new(items)
        .highlight_style(highlight_style)
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, rows[1], list_state);
}

fn draw_find(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    let Mode::Find {
        query,
        results,
        list_state,
    } = &mut app.mode
    else {
        return;
    };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    let input = Paragraph::new(Line::from(vec![
        Span::styled("f ", Style::default().fg(Color::Yellow)),
        Span::styled(
            format!("{}\u{2588}", query),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  (#tag for tag-only)",
            Style::default().fg(Color::Indexed(245)),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .title(format!(" Find ({} results) ", results.len())),
    );
    frame.render_widget(input, rows[0]);

    let items: Vec<ListItem> = results
        .iter()
        .filter_map(|(ext_id, list_name)| {
            let item = app.items.iter().find(|i| i.ext_id == *ext_id)?;
            let mut spans = vec![
                Span::styled(
                    format!("[{:<5}] ", list_name),
                    Style::default().fg(Color::Magenta),
                ),
                Span::styled(
                    format!("{:<6}", item.ext_id),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(format!("{} ", item.title)),
            ];
            if !item.tags.is_empty() {
                spans.push(Span::styled(
                    format!(" [{}]", item.tags.join(" ")),
                    Style::default().fg(Color::Indexed(245)),
                ));
            }
            Some(ListItem::new(Line::from(spans)))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, rows[1], list_state);
}
