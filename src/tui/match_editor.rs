use crossterm::event::KeyEvent;
use tui_textarea::TextArea;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect}, style::{Color, Modifier, Style}, widgets::{Paragraph, Widget}
};
use crate::tui::textarea_ext::TextAreaExt;


struct RegexMatchGrp {
    start_offset: usize,
    end_offset: usize,
    label: String, // computed label (name or 'group N')
    groups: Vec<RegexMatchGrp>,
}

impl RegexMatchGrp {
    fn contains(&self, other: &RegexMatchGrp) -> bool {
        self.start_offset <= other.start_offset && other.end_offset <= self.end_offset
    }

    fn contains_offset(&self, offset: usize) -> bool {
        offset >= self.start_offset && offset < self.end_offset
    }

    fn insert(&mut self, child: RegexMatchGrp) {
        for sub in &mut self.groups {
            if sub.contains(&child) {
                sub.insert(child);
                return;
            }
        }
        self.groups.push(child);
    }
}

pub struct MatchEditorWidget {
    re: Option<regex::bytes::Regex>,
    textarea: TextArea<'static>,
    matches: Vec<RegexMatchGrp>,
}

fn find_group_path(m: &RegexMatchGrp, offset: usize) -> Vec<&RegexMatchGrp>
{
    if !m.contains_offset(offset) {
        return Vec::new();
    }

    let mut path = vec![m];
    for sub in &m.groups {
        if sub.contains_offset(offset) {
            path.extend(find_group_path(sub, offset));
            return path;
        }
    }
    path
}

impl MatchEditorWidget {
    pub fn new() -> Self {
        Self {
            re: None,
            textarea: TextArea::default(),
            matches: Vec::new(),
        }
    }

    pub fn input(&mut self, k: KeyEvent) -> bool {
        self.textarea.input(k)
    }

    fn get_breadcrumb_path(&self) -> Vec<String> {
        let offset = self.textarea.get_flat_offset_from_cursor();
        for m in &self.matches {
            if m.contains_offset(offset) {
                let path = find_group_path(m, offset);
                return path.iter().skip(1).map(|g| g.label.clone()).collect();
            }
        }
        Vec::new()
    }

    // it is rendered on every new text input, but regex is only upated on pattern change
    pub fn update_regex(&mut self, r: Option<regex::bytes::Regex>) {
        self.re = r;
    }

    pub fn eval_regex(&mut self) {
        if let Some(re) = &self.re {
            let lines = self.textarea.lines();
            let text: String = lines.join("\n");

            let grp_names: Vec<Option<&str>> = re.capture_names().collect();

            let mut matches: Vec<RegexMatchGrp> = Vec::new();
            for caps in re.captures_iter(text.as_bytes()) {
                let full_match = caps.get(0).unwrap();
                let mut root_match = RegexMatchGrp {
                    start_offset: full_match.start(),
                    end_offset: full_match.end(),
                    label: String::new(),
                    groups: Vec::new(),
                };

                for (i, grp) in caps.iter().enumerate() {
                    if i == 0 {
                        continue;
                    }
                    let name = grp_names.get(i).and_then(|n| n.and_then(|n| Some(n.to_string())));
                    if let Some(grp_match) = grp {
                        let child = RegexMatchGrp {
                            start_offset: grp_match.start(),
                            end_offset: grp_match.end(),
                            label: if let Some(ref n) = name { n.clone() } else { format!("group {}", i) },
                            groups: Vec::new(),
                        };
                        root_match.insert(child);
                    }
                }

                matches.push(root_match);
            }

            self.textarea.clear_custom_highlight();
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
