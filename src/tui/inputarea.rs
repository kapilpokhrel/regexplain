use crossterm::{clipboard, event::{Event, KeyCode, KeyModifiers}, execute};
use tui_textarea::{TextArea, WrapMode};
use ratatui::{
    layout::Rect, style::{Modifier, Style}, widgets::Widget
};
use crate::tui::textarea_ext::TextAreaExt;
use crate::colorize::ColorGenerator;

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

    pub fn set_pattern(&mut self, p: impl Into<String>) {
        let p = p.into().replace("\n", "");
        if p.is_empty() {
            return
        }
        let col = p.len();
        self.textarea.set_lines(vec![p], (0, col));
    }

    pub fn input(&mut self, e: Event) -> bool {
        if let Event::Key(key) = e  && key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('y') {
            let _ = execute!(
                std::io::stdout(),
                clipboard::CopyToClipboard::to_clipboard_from(
                    self.pattern_str()
                )
            );
            return false;
        }
        self.textarea.input(e)
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
        selected_span: Option<crate::types::PatternSpan>,
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
        let colored_slice = cgen.ratatui_colored_slice(&self.pattern_str(), start, end, fg_bright_factor);
        for (i, span) in colored_slice.iter().enumerate() {
            let mut style = span.style;
            if let Some(additional_s) = additional_style {
                style = style.patch(additional_s);
            }
            let idx = start + i;

            let cr = self.textarea.get_cursor_range_from_offsets(idx, idx + 1);
            self.textarea.custom_highlight(
                cr,
                style,
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
