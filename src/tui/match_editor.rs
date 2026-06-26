use crossterm::event::KeyEvent;
use tui_textarea::TextArea;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect}, style::{Color, Modifier, Style}, widgets::{Paragraph, Widget}
};
use crate::{matcher::eval_regex, tui::textarea_ext::TextAreaExt};
use crate::matcher::RegexMatchGrp;

pub struct MatchEditorWidget {
    re: Option<regex::bytes::Regex>,
    textarea: TextArea<'static>,
    matches: Vec<RegexMatchGrp>,
}

impl MatchEditorWidget {
    pub fn new() -> Self {
        Self {
            re: None,
            textarea: TextArea::default(),
            matches: Vec::new(),
        }
    }

    pub fn get_match_text(&self) -> String {
        self.textarea.lines().join("\n")
    }

    pub fn set_match_text(&mut self, t: impl Into<String>) {
        let lines: Vec<String> = t.into().lines().map(|s| s.to_string()).collect();
        if lines.is_empty() {
            return
        }
        let cursor = (lines.len() - 1, lines.last().unwrap().len());
        self.textarea.set_lines(lines, cursor);
    }

    pub fn input(&mut self, k: KeyEvent) -> bool {
        self.textarea.input(k)
    }

    fn get_breadcrumb_path(&self) -> Vec<String> {
        let offset = self.textarea.get_flat_offset_from_cursor();
        for m in &self.matches {
            if m.contains_offset(offset) {
                let path = m.find_group_path(offset);
                return path.iter().skip(1).map(|g| g.label.clone()).collect();
            }
        }
        Vec::new()
    }

    // it is rendered on every new text input, but regex is only upated on pattern change
    pub fn update_regex(&mut self, r: Option<regex::bytes::Regex>) {
        self.re = r;
    }

    pub fn update(&mut self) {
        if let Some(re) = &self.re {
            self.textarea.clear_custom_highlight();
            let text = self.get_match_text();
            let matches = eval_regex(re, &text);
            for m in &matches {
                Self::highlight_match(&mut self.textarea, m, 1);
            }

            self.matches = matches;
        }
    }

    fn highlight_match(textarea: &mut TextArea, m: &RegexMatchGrp, n: u8) {
        let range = textarea.get_cursor_range_from_offsets(m.start_offset, m.end_offset);
        let colors = [
            Color::Rgb(0, 100, 0),    // Dark Green
            Color::Rgb(100, 100, 0),  // Dark Yellow
            Color::Rgb(100, 0, 0),    // Dark Red
            Color::Rgb(100, 0, 100),  // Dark Magenta
            Color::Rgb(0, 100, 100),  // Dark Cyan
        ];
        let color = colors[(n as usize - 1) % colors.len()];

        textarea.custom_highlight(
            range,
            Style::default().bg(color),
            n,
        );
        for (i, sub) in m.groups.iter().enumerate() {
            MatchEditorWidget::highlight_match(textarea, sub, n + 1 + i as u8);
        }
    }

    fn render_textarea(&self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        self.textarea.render(area, buf);
    }

    //pub fn render_status_line(&self, f: &mut Frame, area: Rect) {
    fn render_status_line(&self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let (row, col) = self.textarea.cursor();
        let offset = self.textarea.get_flat_offset_from_cursor();

        let match_text = if let Some(m_idx) =  self.matches.iter().position(|m| m.contains_offset(offset)) {
            let mut text = format!("Match {}", m_idx);
            let path = self.get_breadcrumb_path();
            if !path.is_empty() {
                text.push_str(" > ");
                text.push_str(&path.join(" > "));
            }
            text
        } else {
            String::new()
        };

        let cursor = format!("({}:{})", row + 1, col + 1);

        let total_lines = self.textarea.lines().len();
        let percentage = if row == 0 {
            "Top "
        } else if row + 1 == total_lines {
            "Bot "
        } else {
            &format!("{}% ", ((row + 1) * 100)/total_lines)
        };
        let status_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Min(1),
                    Constraint::Length(percentage.len() as u16),
                    Constraint::Length(cursor.len() as u16),
                ]
                .as_ref(),
            )
            .split(area);
        let status_style = Style::default().add_modifier(Modifier::REVERSED);

        Paragraph::new(match_text).style(status_style).render(status_chunks[0], buf);
        Paragraph::new(percentage).style(status_style).render(status_chunks[1], buf);
        Paragraph::new(cursor).style(status_style).render(status_chunks[2], buf);
    }
}

impl Widget for &MatchEditorWidget {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized
    {
        let editor_area = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);
        let editor = editor_area[0];
        let status = editor_area[1];
        self.render_textarea(editor, buf);
        self.render_status_line(status, buf);
    }
}
