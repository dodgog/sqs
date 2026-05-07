use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use super::{VisualMarkers, panel_border_style};
use crate::adapter::Item;
use crate::tui::app_state::{FocusedPanel, ListFilter, TuiApp};

pub fn render(
    frame: &mut Frame,
    area: Rect,
    filter: ListFilter,
    items: &[&Item],
    app: &TuiApp,
    all_list_names: &[String],
    focused: bool,
) {
    let cursor_idx = app.task_list_state.selected();
    let cursor_list = cursor_idx
        .and_then(|i| items.get(i))
        .map(|i| i.list.as_str());
    let title = match &filter {
        ListFilter::Single(name) => format!(" {name} ({}) ", items.len()),
        ListFilter::All => match cursor_list {
            Some(l) => format!(" All ({}) │ in: {l} ", items.len()),
            None => format!(" All ({}) ", items.len()),
        },
    };

    // Keep the cursor highlight visible whenever the user is interacting with
    // an entity — task list itself OR the detail preview of the same entity.
    let show_cursor = focused || app.focused_panel == FocusedPanel::Detail;
    let cursor = if show_cursor { cursor_idx } else { None };
    let markers = VisualMarkers::for_pane(
        cursor,
        app.visual_anchor_for(FocusedPanel::TaskList),
        app.visual_cursor_for(FocusedPanel::TaskList),
    );

    let is_all = matches!(filter, ListFilter::All);
    let mut list_items: Vec<ListItem> = Vec::new();

    if is_all {
        for list_name in all_list_names {
            list_items.push(ListItem::new(Line::from(Span::styled(
                format!("  {list_name}"),
                Style::default().fg(Color::Indexed(245)),
            ))));
            for (idx, item) in items.iter().enumerate() {
                if item.list == *list_name {
                    list_items.push(item_row(item, idx, &markers));
                }
            }
        }
    } else {
        for (idx, item) in items.iter().enumerate() {
            list_items.push(item_row(item, idx, &markers));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::styled(title, Style::default().fg(Color::White)))
        .border_style(panel_border_style(focused));

    let list = List::new(list_items).block(block);
    frame.render_widget(list, area);
}

fn item_row<'a>(item: &'a Item, row: usize, markers: &VisualMarkers) -> ListItem<'a> {
    let spans = vec![
        Span::raw(markers.cursor_glyph(row).to_string()),
        Span::raw(markers.anchor_glyph(row).to_string()),
        Span::raw(" "),
        Span::styled(
            format!("{:<6}", item.ext_id),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw(item.title.clone()),
    ];
    let mut li = ListItem::new(Line::from(spans));
    if markers.should_highlight(row) {
        li = li.style(Style::default().bg(Color::DarkGray));
    }
    li
}
