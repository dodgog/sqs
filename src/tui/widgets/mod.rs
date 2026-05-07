pub mod add_form;
pub mod add_sublist;
pub mod detail;
pub mod sidebar;
pub mod status_bar;
pub mod tag_picker;
pub mod tags;
pub mod task_list;

use ratatui::style::{Color, Style};

pub fn panel_border_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Indexed(245))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VisualMarkers {
    pub cursor: Option<usize>,
    pub anchor: Option<usize>,
    pub range: Option<(usize, usize)>,
}

impl VisualMarkers {
    pub fn for_pane(
        cursor: Option<usize>,
        visual_anchor: Option<usize>,
        visual_cursor: Option<usize>,
    ) -> Self {
        let range = match (visual_anchor, visual_cursor) {
            (Some(a), Some(c)) => Some((a.min(c), a.max(c))),
            _ => None,
        };
        Self {
            cursor: visual_cursor.or(cursor),
            anchor: visual_anchor,
            range,
        }
    }

    /// Cursor glyph: `>` on the cursor row.
    pub fn cursor_glyph(&self, row: usize) -> &'static str {
        if Some(row) == self.cursor { ">" } else { " " }
    }

    /// Anchor glyph: `o` on the anchor row in visual mode. Renders even when
    /// anchor and cursor coincide, so a single-row visual selection shows
    /// `>o` together.
    pub fn anchor_glyph(&self, row: usize) -> &'static str {
        match self.anchor {
            Some(a) if row == a => "o",
            _ => " ",
        }
    }

    pub fn is_in_range(&self, row: usize) -> bool {
        match self.range {
            Some((s, e)) => row >= s && row <= e,
            None => false,
        }
    }

    /// Whether the row should receive the dark-gray background. True for any
    /// row in the active visual range, and always for the cursor row.
    pub fn should_highlight(&self, row: usize) -> bool {
        self.is_in_range(row) || Some(row) == self.cursor
    }
}
