use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};

use super::panel_border_style;
use crate::adapter::Item;
use crate::tui::app_state::ListFilter;

#[allow(clippy::too_many_arguments)]
pub fn render(
    frame: &mut Frame,
    area: Rect,
    filter: ListFilter,
    items: &[&Item],
    selected: Option<usize>,
    visual_range: Option<(usize, usize)>,
    all_list_names: &[String],
    focused: bool,
) {
    let title = match &filter {
        ListFilter::Single(name) => format!(" {name} ({}) ", items.len()),
        ListFilter::All => format!(" All ({}) ", items.len()),
    };

    let is_all = matches!(filter, ListFilter::All);

    let mut list_items: Vec<ListItem> = Vec::new();
    let mut row_to_item: Vec<Option<usize>> = Vec::new();

    if is_all {
        // Show all lists in sidebar order, including empty ones
        for list_name in all_list_names {
            // Heading
            list_items.push(ListItem::new(Line::from(Span::styled(
                list_name.to_string(),
                Style::default().fg(Color::Indexed(245)),
            ))));
            row_to_item.push(None);

            // Items in this list
            for (idx, item) in items.iter().enumerate() {
                if item.list == *list_name {
                    list_items.push(item_row(item, idx, visual_range));
                    row_to_item.push(Some(idx));
                }
            }
        }
    } else {
        for (idx, item) in items.iter().enumerate() {
            list_items.push(item_row(item, idx, visual_range));
            row_to_item.push(Some(idx));
        }
    }

    let display_selected = selected.and_then(|item_idx| {
        row_to_item
            .iter()
            .position(|mapping| *mapping == Some(item_idx))
    });

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::styled(title, Style::default().fg(Color::White)))
        .border_style(panel_border_style(focused));

    let highlight_style = Style::default().bg(Color::DarkGray);

    let list = List::new(list_items)
        .block(block)
        .highlight_style(highlight_style)
        .highlight_symbol("> ");

    let mut state = ListState::default().with_selected(display_selected);
    frame.render_stateful_widget(list, area, &mut state);
}

fn item_row<'a>(item: &'a Item, idx: usize, visual_range: Option<(usize, usize)>) -> ListItem<'a> {
    let spans = vec![
        Span::styled(
            format!("{:<6}", item.ext_id),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw(&item.title),
    ];

    let mut li = ListItem::new(Line::from(spans));

    if let Some((start, end)) = visual_range
        && idx >= start
        && idx <= end
    {
        li = li.style(Style::default().bg(Color::DarkGray));
    }

    li
}
