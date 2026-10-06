//! The minimap's model: text turned into colored runs, and the math that maps
//! between the editor's scroll position and the minimap.

use std::ops::Range;

use gpui_kit::{HighlightStyle, Hsla};

/// Height of one text line in the minimap, in pixels.
pub const LINE_HEIGHT: f32 = 2.0;
/// Width of one character column, in pixels.
pub const CHAR_WIDTH: f32 = 1.0;
/// Columns past this aren't drawn; the minimap is narrower anyway.
const MAX_COLUMNS: usize = 160;
const TAB_COLUMNS: usize = 4;

/// One stretch of non-space characters on a line, all the same color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Run {
    pub column: u16,
    pub len: u16,
    pub color: Hsla,
}

/// Splits `text` into lines of colored runs. `styles` are the syntax
/// highlights (byte ranges, sorted); text they don't cover uses `default`.
pub fn build_lines(text: &str, styles: &[(Range<usize>, HighlightStyle)], default: Hsla) -> Vec<Vec<Run>> {
    let mut lines = Vec::new();
    let mut style_ix = 0;
    let mut offset = 0;

    for line in text.split('\n') {
        let mut runs: Vec<Run> = Vec::new();
        let mut column = 0;
        for (byte, ch) in line.char_indices() {
            if column >= MAX_COLUMNS {
                break;
            }
            if ch == '\t' {
                column += TAB_COLUMNS - column % TAB_COLUMNS;
                continue;
            }
            if ch.is_whitespace() {
                column += 1;
                continue;
            }
            let at = offset + byte;
            while style_ix < styles.len() && styles[style_ix].0.end <= at {
                style_ix += 1;
            }
            let color = styles
                .get(style_ix)
                .filter(|(range, _)| range.start <= at)
                .and_then(|(_, style)| style.color)
                .unwrap_or(default);
            match runs.last_mut() {
                Some(run) if run.color == color && usize::from(run.column + run.len) == column => run.len += 1,
                _ => runs.push(Run {
                    column: column as u16,
                    len: 1,
                    color,
                }),
            }
            column += 1;
        }
        lines.push(runs);
        offset += line.len() + 1;
    }
    lines
}

/// Where the minimap is scrolled and where the viewport box sits, in pixels
/// from the top of the minimap pane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub scroll: f32,
    pub box_top: f32,
    pub box_height: f32,
}

/// The editor's position, in its own rows: `top` is the first visible row and
/// `visible` how many fit. With word wrap off, rows are buffer lines.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub total: f32,
    pub top: f32,
    pub visible: f32,
}

impl Viewport {
    /// When the document is taller than the pane, the minimap scrolls in step
    /// with the editor, so the box moves a little slower than the lines under it.
    pub fn view(&self, pane_height: f32) -> View {
        let content = self.total * LINE_HEIGHT;
        let range = (content - pane_height).max(0.0);
        let max_top = (self.total - self.visible).max(1.0);
        let scroll = (self.top / max_top).clamp(0.0, 1.0) * range;
        View {
            scroll,
            box_top: self.top * LINE_HEIGHT - scroll,
            box_height: (self.visible * LINE_HEIGHT).max(LINE_HEIGHT),
        }
    }

    /// How many pixels the box moves per row the editor scrolls.
    fn box_speed(&self, pane_height: f32) -> f32 {
        let range = (self.total * LINE_HEIGHT - pane_height).max(0.0);
        let speed = LINE_HEIGHT - range / (self.total - self.visible).max(1.0);
        if speed > 0.1 { speed } else { LINE_HEIGHT }
    }

    /// The top row that puts the box's top edge at `box_top`, for dragging.
    pub fn top_for_box(&self, box_top: f32, pane_height: f32) -> f32 {
        self.clamp_top(box_top / self.box_speed(pane_height))
    }

    /// The top row that centers the line under `y` in the editor, for clicks.
    pub fn top_centering(&self, y: f32, pane_height: f32) -> f32 {
        let line = (y + self.view(pane_height).scroll) / LINE_HEIGHT;
        self.clamp_top(line - self.visible / 2.0)
    }

    fn clamp_top(&self, top: f32) -> f32 {
        top.clamp(0.0, (self.total - 1.0).max(0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::hsla;

    #[test]
    fn runs_follow_words_tabs_and_syntax_colors() {
        let red = hsla(0.0, 1.0, 0.5, 1.0);
        let text = "let x = 1;\n\tfoo";
        let keyword = HighlightStyle {
            color: Some(red),
            ..HighlightStyle::default()
        };
        let lines = build_lines(text, &[(0..3, keyword)], Hsla::white());

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0][0], Run { column: 0, len: 3, color: red });
        let columns: Vec<_> = lines[0].iter().map(|run| (run.column, run.len)).collect();
        assert_eq!(columns, [(0, 3), (4, 1), (6, 1), (8, 2)]);
        assert_eq!(lines[1][0].column, 4, "a tab moves to the next tab stop");
    }

    #[test]
    fn a_short_document_does_not_scroll_the_minimap() {
        let viewport = Viewport {
            total: 50.0,
            top: 10.0,
            visible: 30.0,
        };
        let view = viewport.view(400.0);
        assert_eq!(view.scroll, 0.0);
        assert_eq!(view.box_top, 20.0);
        assert_eq!(view.box_height, 60.0);
    }

    #[test]
    fn a_long_document_scrolls_the_minimap_in_step() {
        let viewport = Viewport {
            total: 1000.0,
            top: 0.0,
            visible: 40.0,
        };
        assert_eq!(viewport.view(400.0).scroll, 0.0);

        let bottom = Viewport { top: 960.0, ..viewport };
        let view = bottom.view(400.0);
        assert_eq!(view.scroll, 1600.0, "fully scrolled at the end");
        assert_eq!(view.box_top + view.box_height, 400.0, "the box sits at the bottom of the pane");
    }

    #[test]
    fn dragging_the_box_round_trips_to_the_same_row() {
        let viewport = Viewport {
            total: 1000.0,
            top: 300.0,
            visible: 40.0,
        };
        let box_top = viewport.view(400.0).box_top;
        assert!((viewport.top_for_box(box_top, 400.0) - 300.0).abs() < 0.01);
    }

    #[test]
    fn clicking_centers_the_line_under_the_pointer() {
        let viewport = Viewport {
            total: 100.0,
            top: 0.0,
            visible: 20.0,
        };
        assert_eq!(viewport.top_centering(100.0, 400.0), 40.0);
        assert_eq!(viewport.top_centering(0.0, 400.0), 0.0, "never above the first line");
    }
}
