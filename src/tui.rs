use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget, Wrap},
};
use regex::bytes::Regex;

use crate::colorize::{ColorGenerator, Colorizer};
use crate::convert;
use crate::desc::{DescGenerator, DescNode, Describer};
use crate::tree::{Node, TreeWidget};
use tui_textarea::TextArea;

type CursorRange = ((usize, usize), (usize, usize));
struct TextMatch {
    range: CursorRange,
    groups: Vec<(CursorRange, Option<String>)> // vector index + 1 will be group index
}

#[derive(PartialEq, Clone, Copy)]
enum Focus {
    PatternInput,
    TextToMatch,
    DescTree,
}

impl Focus {
    fn next(self) -> Self {
        match self {
            Self::PatternInput => Self::TextToMatch,
            Self::TextToMatch => Self::DescTree,
            Self::DescTree => Self::PatternInput,
        }
    }
}

struct App {
    focus: Focus,

    input: String,
    inp_cursor_pos: usize,

    tree: TreeWidget,
    textarea: TextArea<'static>,

    re: Option<Regex>,
    pattern_line: Line<'static>,
    error: Option<String>,

    matches: Vec<TextMatch>,
    // Keep last parsed pattern & colors so tree selection can update the input line styling.
    last_cgen: Option<ColorGenerator>,
    last_desc: Option<crate::desc::DescNode>,
}

impl App {
    fn new() -> Self {
        Self {
            focus: Focus::PatternInput,
            input: String::new(),
            inp_cursor_pos: 0,
            tree: TreeWidget::new(),
            textarea: TextArea::default(),
            re: None,
            pattern_line: Line::default(),
            error: None,
            matches: Vec::new(),
            last_cgen: None,
            last_desc: None,
        }
    }

    fn reparse(&mut self) {
        if self.input.is_empty() {
            self.pattern_line = Line::default();
            self.error = None;
            self.tree.set_nodes(vec![]);
            self.last_cgen = None;
            return;
        }
        match convert::parse_and_convert(&self.input) {
            Ok(form) => {
                self.error = None;
                let mut cgen = ColorGenerator::new();
                cgen.colorize(&form.root);

                let root = DescGenerator::new().describe(form.root);

                let nodes = desc_to_nodes(&root, &form.pattern, &cgen);
                self.tree.set_nodes(nodes);

                // Store last pattern + color generator + desc root for tree lookups.
                self.last_cgen = Some(cgen);
                self.last_desc = Some(root);

                self.re = Regex::new(&self.input).ok(); // we have already checked for the error
                self.update_pattern_line();
            }
            Err(e) => {
                self.error = Some(e.to_string());
                self.pattern_line = Line::default();
                self.tree.set_nodes(vec![]);
                self.last_cgen = None;
            }
        }
    }

    fn eval_regex(&mut self) {
        if let Some(re) = &self.re {
            let lines = self.textarea.lines();
            let text: String = lines.join("\n");

            let get_cursor_pos = |x: usize| {
                let mut x = x;
                let mut row = 0usize;
                let mut col = 0;
                for line in lines {
                    if x <= line.len() {
                        col = x;
                        break;
                    } else {
                        row += 1;
                        x -= line.len() + 1; // +1 for the newline
                    }
                }
                (row, col)
            };

            let get_cursor_range = |m: &regex::bytes::Match| {
                (
                    get_cursor_pos(m.start()),
                    get_cursor_pos(m.end())
                ) as CursorRange
            };

            let grp_names: Vec<Option<&str>> = re.capture_names().collect();

            let mut matches: Vec<TextMatch> = Vec::new();
            for caps in re.captures_iter(text.as_bytes()) {
                let full_match = caps.get(0).unwrap();
                let mut groups = Vec::new();

                for (i, grp) in caps.iter().enumerate() {
                    let name = grp_names.get(i).and_then(|n| n.and_then(|n| Some(n.to_string())));
                    if let Some(grp_match) = grp {
                        groups.push((
                            get_cursor_range(&grp_match),
                            name
                        ))
                    }
                }

                matches.push(TextMatch{
                    range: get_cursor_range(&full_match),
                    groups
                })
            }
            self.matches = matches;
        }
    }

    fn get_selected_span(&self) -> Option<crate::types::Span> {
        if let Some(d) = &self.last_desc {
            let mut cur = d;
            for &idx in self.tree.selected_path() {
                if let Some(n) = cur.nested_items.get(idx) {
                    cur = n;
                } else {
                    break;
                }
            }
            Some(cur.span)
        } else {
            None
        }
    }
    fn update_pattern_line(&mut self) {
        if let (Some(c), Some(span)) = (
            &self.last_cgen,
            self.get_selected_span(),
        ) {
            self.pattern_line = self.render_pattern_line(&self.input, c, Some(span));
        }
    }

    /// Render the whole pattern line but make the selected node's span bold (if desctree is focused)
    fn render_pattern_line(
        &self,
        pattern: &str,
        cgen: &ColorGenerator,
        selected_span: Option<crate::types::Span>,
    ) -> Line<'static> {
        let len = pattern.len();
        if len == 0 {
            return Line::default();
        }
        if let Some(span) = selected_span
            && self.focus == Focus::DescTree
        {
            let s = span.start;
            let e = span.end;
            // Validate bounds
            if s >= e || e > len {
                return render_slice(pattern, 0, len, cgen, 1.0);
            }

            let mut spans: Vec<Span<'static>> = Vec::new();
            // before (dim slightly)
            spans.extend(render_slice(pattern, 0, s, cgen, 0.75).spans);
            // selected (make bold + brighter)
            let sel_line = render_slice(pattern, s, e, cgen, 1.5);
            for sp in sel_line.spans.into_iter() {
                let new_style = sp.style.add_modifier(Modifier::BOLD);
                spans.push(Span::styled(sp.content.clone(), new_style));
            }
            // after (dim slightly)
            spans.extend(render_slice(pattern, e, len, cgen, 0.75).spans);
            Line::from(spans)
        } else {
            render_slice(pattern, 0, len, cgen, 1.0)
        }
    }

    fn input_insert(&mut self, c: char) {
        self.input.insert(self.inp_cursor_pos, c);
        self.inp_cursor_pos += c.len_utf8();
        self.reparse();
    }
    fn input_backspace(&mut self) {
        if self.inp_cursor_pos > 0 {
            let prev = self.input[..self.inp_cursor_pos]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input.remove(prev);
            self.inp_cursor_pos = prev;
            self.reparse();
        }
    }
    fn input_left(&mut self) {
        if self.inp_cursor_pos > 0 {
            self.inp_cursor_pos = self.input[..self.inp_cursor_pos]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }
    fn input_right(&mut self) {
        if self.inp_cursor_pos < self.input.len() {
            self.inp_cursor_pos += self.input[self.inp_cursor_pos..]
                .chars()
                .next()
                .unwrap()
                .len_utf8();
        }
    }
}

fn desc_to_nodes(node: &DescNode, pattern: &str, cgen: &ColorGenerator) -> Vec<Node> {
    let children: Vec<Node> = node
        .nested_items
        .iter()
        .flat_map(|child| desc_to_nodes(child, pattern, cgen))
        .collect();

    if node.desc.is_empty() {
        // Transparent concat node — hoist children up
        return children;
    }

    let span = node.span;
    let mut spans = vec![Span::raw("`")];
    spans.extend(render_slice(pattern, span.start, span.end, cgen, 1.0).spans);
    spans.push(Span::raw(format!("` {}", node.desc)));

    vec![Node::new(Line::from(spans), children)]
}

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

fn render_slice(
    pattern: &str,
    start: usize,
    end: usize,
    cgen: &ColorGenerator,
    fg_bright_factor: f32,
) -> Line<'static> {
    if start >= end {
        return Line::default();
    }
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut seg = start;
    let (mut cfg, mut cbg) = cgen.char_color(start);
    cfg = brighten_fg(cfg, fg_bright_factor);
    for (rel, _) in pattern[start..end].char_indices().skip(1) {
        let abs = start + rel;
        let (fg, bg) = cgen.char_color(abs);
        let fg_b = brighten_fg(fg, fg_bright_factor);
        if fg_b != cfg || bg != cbg {
            push_span(pattern, seg, abs, cfg, cbg, &mut spans);
            seg = abs;
            cfg = fg_b;
            cbg = bg;
        }
    }
    push_span(pattern, seg, end, cfg, cbg, &mut spans);
    Line::from(spans)
}

fn push_span(
    pat: &str,
    start: usize,
    end: usize,
    fg: Option<[f32; 3]>,
    bg: Option<[f32; 3]>,
    out: &mut Vec<Span<'static>>,
) {
    let text = pat[start..end].to_string();
    if text.is_empty() {
        return;
    }
    let mut style = Style::default();
    if let Some(f) = fg {
        style = style.fg(to_color(f));
    }
    if let Some(b) = bg {
        style = style.bg(to_color(b));
    }
    out.push(Span::styled(text, style));
}

fn wrapped_lines(s: &str, width: u16) -> u16 {
    if width == 0 || s.is_empty() {
        return 1;
    }
    let mut rows = 1u16;
    let mut col = 0u16;
    for ch in s.chars() {
        if ch == '\n' {
            rows += 1;
            col = 0;
        } else {
            col += 1;
            if col >= width {
                rows += 1;
                col = 0;
            }
        }
    }
    rows
}

fn focused_block<'a>(title: &'a str, focused: bool) -> Block<'a> {
    let style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    Block::bordered().title(title).border_style(style)
}

fn render_pattern_input(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::PatternInput;
    let content: Line = if app.error.is_some() {
        Line::styled(app.input.clone(), Style::default().fg(Color::Red))
    } else if app.pattern_line.spans.is_empty() {
        Line::raw(app.input.clone())
    } else {
        app.pattern_line.clone()
    };
    Paragraph::new(content)
        .block(focused_block("Pattern  (Esc quit · Shift+Tab cycle)", focused))
        .wrap(Wrap { trim: false })
        .render(area, f.buffer_mut());

    if focused {
        let iw = area.width.saturating_sub(2);
        let ci = app.input[..app.inp_cursor_pos].chars().count() as u16;
        let (cr, cc) = if let Some(q) = ci.checked_div(iw) {
            (q, ci % iw)
        } else {
            (0, ci)
        };
        f.set_cursor_position((area.x + 1 + cc, area.y + 1 + cr));
    }
}

fn render_status_line(f: &mut Frame, app: &mut App, area: Rect) {
    let (row, col) = app.textarea.cursor();

    // inline function to get check if the cursor (usize, usize) is inside a cusrorRange
    let is_inside_range = |r: &CursorRange| -> bool {
        (row >= r.0.0 && row <= r.1.0) && (col >= r.0.1 && col <= r.1.1)
    };

    let mut match_text = String::new();
    for (i,m) in app.matches.iter().enumerate() {
        if is_inside_range(&m.range) {
            match_text = format!("Match {}", i);
            let mut grp_iter = m.groups.iter().enumerate();
            grp_iter.next();
            for (i, g) in grp_iter {
                if is_inside_range(&g.0) {
                    let grp_str = if let Some(ref n) = g.1 {
                        format!(", group {}", n)
                    } else {
                        format!(", group {}", i)
                    };
                    match_text.push_str(grp_str.as_str());
                    break;
                }
            }
            break;
        }
    }

    let cursor = format!("({}:{})", row + 1, col + 1);

    let total_lines = app.textarea.lines().len();
    let percentage = if row == 0 {
        "Top "
    } else if row + 1 == total_lines {
        "Bot "
    } else {
        &format!("{}% ", ((row + 1) * 100)/app.textarea.lines().len())
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
    f.render_widget(Paragraph::new(match_text).style(status_style), status_chunks[0]);
    f.render_widget(Paragraph::new(percentage).style(status_style), status_chunks[1]);
    f.render_widget(Paragraph::new(cursor).style(status_style), status_chunks[2]);
}

fn render_textarea(f: &mut Frame, app: &mut App, area: Rect) {
    for m in app.matches.iter() {
        app.textarea.custom_highlight(
            m.range,
            Style::default().bg(Color::LightBlue),
            0
        );
        for (i, g) in m.groups.iter().enumerate() {
            app.textarea.custom_highlight(
                g.0,
                Style::default().bg(Color::Green),
                (i + 1) as u8
            );
        }
    }
    f.render_widget(&app.textarea, area);
}

fn render_text_to_match(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::TextToMatch;
    let block = focused_block("Text to match", focused);
    let inner = block.inner(area);
    block.render(area, f.buffer_mut());

    if inner.width < 2 || inner.height < 2 {
        return;
    }

    let editor_area = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);
    let editor = editor_area[0];
    let status = editor_area[1];
    render_textarea(f, app, editor);
    render_status_line(f, app, status);
}

fn render_tree_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::DescTree;
    let (sel, total) = app.tree.selected_index_total();
    let title = match sel {
        Some(i) => format!("Description: {}/{}", i, total),
        None => "Description".to_string(),
    };
    let style = if focused { Style::default().fg(Color::Yellow) } else { Style::default() };
    let block = Block::bordered().title(title).border_style(style);
    let inner = block.inner(area);
    block.render(area, f.buffer_mut());
    app.tree.render(inner, f.buffer_mut(), focused);
}

fn ui(f: &mut Frame, app: &mut App) {
    let sides = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(f.area());

    let iw = sides[0].width.saturating_sub(2);
    let ih = (wrapped_lines(&app.input, iw) + 2).max(3);
    let left = Layout::vertical([Constraint::Length(ih), Constraint::Min(0)]).split(sides[0]);

    render_pattern_input(f, app, left[0]);
    render_text_to_match(f, app, left[1]);
    render_tree_panel(f, app, sides[1]);
}

pub fn run() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal);
    ratatui::restore();
    result
}

fn run_app(terminal: &mut ratatui::DefaultTerminal) -> io::Result<()> {
    let mut app = App::new();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if !event::poll(std::time::Duration::from_millis(50))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) | (KeyModifiers::CONTROL, KeyCode::Char('c')) => break,
            (_, KeyCode::BackTab) => {
                app.focus = app.focus.next();
                app.update_pattern_line();
                continue;
            }
            _ => {}
        }

        match app.focus {
            Focus::PatternInput => match (key.modifiers, key.code) {
                (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => app.input_insert(c),
                (_, KeyCode::Backspace) => app.input_backspace(),
                (_, KeyCode::Left) => app.input_left(),
                (_, KeyCode::Right) => app.input_right(),
                (_, KeyCode::Home) => app.inp_cursor_pos = 0,
                (_, KeyCode::End) => app.inp_cursor_pos = app.input.len(),
                _ => {}
            },

            Focus::TextToMatch => {
                if !app.textarea.input(key) {
                    continue;
                }
                app.eval_regex();
            }

            Focus::DescTree => match (key.modifiers, key.code) {
                (KeyModifiers::CONTROL, KeyCode::Char('n')) => app.tree.scroll_down(),
                (KeyModifiers::CONTROL, KeyCode::Char('j')) => app.tree.scroll_up(),
                (_, KeyCode::Char('j') | KeyCode::Down) => {
                    app.tree.select_down();
                    app.update_pattern_line();
                }
                (_, KeyCode::Char('k') | KeyCode::Up) => {
                    app.tree.select_up();
                    app.update_pattern_line();
                }
                (_, KeyCode::Char('h') | KeyCode::Left) => {
                    app.tree.select_left();
                    app.update_pattern_line();
                }
                (_, KeyCode::Char('l') | KeyCode::Right) => {
                    app.tree.select_right();
                    app.update_pattern_line();
                }
                _ => {}
            },
        }
    }
    Ok(())
}
