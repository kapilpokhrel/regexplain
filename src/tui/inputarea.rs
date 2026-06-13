use crossterm::event::KeyEvent;
use tui_textarea::{TextArea, WrapMode};
use ratatui::{
    layout::Rect, style::{Color, Modifier, Style}, widgets::Widget
};
use crate::tui::textarea_ext::TextAreaExt;
use crate::colorize::ColorGenerator;


fn to_color(rgb: [f32; 3]) -> Color {
    let b = |v: f32| (v * 255.0).clamp(0.0, 255.0) as u8;
    Color::Rgb(b(rgb[0]), b(rgb[1]), b(rgb[2]))
}

fn brighten_fg(fg: Option<[f32; 3]>, factor: f32) -> Option<[f32; 3]> {
    fg.map(|c| {
        [
            (c[0] * factor).min(1.0),
            (c[1] * factor).min(1.0),
            (c[2] * factor).min(1.0),
        ]
    })
}

pub struct InputLineWidget {
    textarea: TextArea<'static>,
}

impl InputLineWidget {
    pub fn new() -> Self {
        let mut t = TextArea::default();
        t.set_wrap_mode(WrapMode::Glyph);
        Self {
            textarea: t,
        }
    }

    pub fn input(&mut self, k: KeyEvent) -> bool {
        self.textarea.input(k)
    }

    pub fn pattern_str(&self) -> String {
        self.textarea.lines().join("")
    }

    pub fn clear_highlight(&mut self) {
        self.textarea.clear_custom_highlight();
    }

    /// Render the whole pattern line but make the selected node's span bold (if desctree is focused)
    pub fn render_input_line(
        &mut self,
        cgen: &ColorGenerator,
        selected_span: Option<crate::types::Span>,
    ) {
        let pattern = self.pattern_str();
        let len = pattern.len();
        if len == 0 {
            return;
        }
        self.clear_highlight();
        if let Some(span) = selected_span {
            let s = span.start;
            let e = span.end;
            // Validate bounds
            if s >= e || e > len {
                self.render_slice(0, len, cgen, 1.0, None);
            }

            // before (dim slightly)
            self.render_slice(0, s, cgen, 0.75, None);
            // selected (make bold + brighter)
            self.render_slice(s, e, cgen, 1.5, Some(Style::default().add_modifier(Modifier::BOLD)));
            // after (dim slightly)
            self.render_slice(e, len, cgen, 0.75, None);
        } else {
            self.render_slice(0, len, cgen, 1.0, None)
        }
    }

    fn render_slice(
        &mut self,
        start: usize,
        end: usize,
        cgen: &ColorGenerator,
        fg_bright_factor: f32,
        additional_style: Option<Style>
    ) {
        if start >= end {
            return;
        }
        for idx in start..end {
            let (fg, bg) = cgen.char_color(idx);
            let fg_b = brighten_fg(fg, fg_bright_factor);
            let mut s = Style::default();
            if let Some(f) = fg_b {
                s = s.fg(to_color(f));
            }
            if let Some(b) = bg {
                s = s.bg(to_color(b));
            }
            if let Some(additional_s) = additional_style {
                s = s.patch(additional_s);
            }
            let cr = self.textarea.get_cursor_range_from_offsets(idx, idx + 1);
            self.textarea.custom_highlight(
                cr,
                s,
                1
            );
        }
    }
}

impl Widget for &InputLineWidget {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized
    {
        self.textarea.render(area, buf)
    }
}
