use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget, Wrap},
    Frame,
};

use crate::colorize::{ColorGenerator, Colorizer};
use crate::convert;
use crate::desc::{DescGenerator, DescNode, Describer};
use crate::tree::{Node, TreeWidget};

// ── Focus ─────────────────────────────────────────────────────────────────────

#[derive(PartialEq, Clone, Copy)]
enum Focus { PatternInput, TextToMatch, DescTree }

impl Focus {
    fn next(self) -> Self {
        match self {
            Self::PatternInput => Self::TextToMatch,
            Self::TextToMatch  => Self::DescTree,
            Self::DescTree     => Self::PatternInput,
        }
    }
}

// ── App state ─────────────────────────────────────────────────────────────────

struct App {
    focus: Focus,

    input: String,
    cursor_pos: usize,

    text: String,
    text_cursor: usize,
    text_vscroll: usize,
    text_hscroll: usize,

    tree: TreeWidget,

    pattern_line: Line<'static>,
    error: Option<String>,
}

impl App {
    fn new() -> Self {
        Self {
            focus: Focus::PatternInput,
            input: String::new(),
            cursor_pos: 0,
            text: String::new(),
            text_cursor: 0,
            text_vscroll: 0,
            text_hscroll: 0,
            tree: TreeWidget::new(),
            pattern_line: Line::default(),
            error: None,
        }
    }

    fn reparse(&mut self) {
        if self.input.is_empty() {
            self.pattern_line = Line::default();
            self.error = None;
            self.tree.set_nodes(vec![]);
            return;
        }
        match convert::parse_and_convert(&self.input) {
            Ok(form) => {
                self.error = None;
                let mut cgen = ColorGenerator::new();
                cgen.colorize(&form.root);
                self.pattern_line = render_slice(&form.pattern, 0, form.pattern.len(), &cgen);
                let root = DescGenerator::new().describe(form.root);
                let nodes = desc_to_nodes(&root, &form.pattern, &cgen);
                self.tree.set_nodes(nodes);
            }
            Err(e) => {
                self.error = Some(e.to_string());
                self.pattern_line = Line::default();
                self.tree.set_nodes(vec![]);
            }
        }
    }

    fn input_insert(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
        self.reparse();
    }
    fn input_backspace(&mut self) {
        if self.cursor_pos > 0 {
            let prev = self.input[..self.cursor_pos]
                .char_indices().last().map(|(i, _)| i).unwrap_or(0);
            self.input.remove(prev);
            self.cursor_pos = prev;
            self.reparse();
        }
    }
    fn input_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos = self.input[..self.cursor_pos]
                .char_indices().last().map(|(i, _)| i).unwrap_or(0);
        }
    }
    fn input_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos += self.input[self.cursor_pos..].chars().next().unwrap().len_utf8();
        }
    }

    fn text_insert(&mut self, c: char) {
        self.text.insert(self.text_cursor, c);
        self.text_cursor += c.len_utf8();
    }
    fn text_newline(&mut self) {
        self.text.insert(self.text_cursor, '\n');
        self.text_cursor += 1;
    }
    fn text_backspace(&mut self) {
        if self.text_cursor > 0 {
            let prev = self.text[..self.text_cursor]
                .char_indices().last().map(|(i, _)| i).unwrap_or(0);
            self.text.remove(prev);
            self.text_cursor = prev;
        }
    }
}

// ── Build Node tree from DescNode ─────────────────────────────────────────────

fn desc_to_nodes(node: &DescNode, pattern: &str, cgen: &ColorGenerator) -> Vec<Node> {
    let children: Vec<Node> = node.nested_items.iter()
        .flat_map(|child| desc_to_nodes(child, pattern, cgen))
        .collect();

    if node.desc.is_empty() {
        // Transparent concat node — hoist children up
        return children;
    }

    let span = node.span;
    let mut spans = vec![Span::raw("`")];
    spans.extend(render_slice(pattern, span.start, span.end, cgen).spans);
    spans.push(Span::raw(format!("` {}", node.desc)));

    vec![Node::new(Line::from(spans), children)]
}

// ── Color helpers ─────────────────────────────────────────────────────────────

fn to_color(rgb: [f32; 3]) -> Color {
    let b = |v: f32| (v * 255.0).clamp(0.0, 255.0) as u8;
    Color::Rgb(b(rgb[0]), b(rgb[1]), b(rgb[2]))
}

fn render_slice(pattern: &str, start: usize, end: usize, cgen: &ColorGenerator) -> Line<'static> {
    if start >= end { return Line::default(); }
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut seg = start;
    let (mut cfg, mut cbg) = cgen.char_color(start);
    for (rel, _) in pattern[start..end].char_indices().skip(1) {
        let abs = start + rel;
        let (fg, bg) = cgen.char_color(abs);
        if fg != cfg || bg != cbg {
            push_span(pattern, seg, abs, cfg, cbg, &mut spans);
            seg = abs; cfg = fg; cbg = bg;
        }
    }
    push_span(pattern, seg, end, cfg, cbg, &mut spans);
    Line::from(spans)
}

fn push_span(pat: &str, start: usize, end: usize,
             fg: Option<[f32; 3]>, bg: Option<[f32; 3]>,
             out: &mut Vec<Span<'static>>) {
    let text = pat[start..end].to_string();
    if text.is_empty() { return; }
    let mut style = Style::default();
    if let Some(f) = fg { style = style.fg(to_color(f)); }
    if let Some(b) = bg { style = style.bg(to_color(b)); }
    out.push(Span::styled(text, style));
}

// ── Scrollbars ────────────────────────────────────────────────────────────────

fn draw_hscrollbar(buf: &mut Buffer, x: u16, y: u16, width: u16, total: usize, offset: usize) {
    if width == 0 { return; }
    let track = Style::default().fg(Color::DarkGray);
    let thumb = Style::default().fg(Color::Gray);
    if total <= width as usize {
        for col in 0..width {
            if let Some(c) = buf.cell_mut((x + col, y)) { c.set_char('─'); c.set_style(track); }
        }
        return;
    }
    let tw = ((width as f64 * width as f64) / total as f64).ceil() as u16;
    let tw = tw.max(1).min(width);
    let range = total - width as usize;
    let tl = ((offset as f64 / range as f64) * (width - tw) as f64).round() as u16;
    for col in 0..width {
        let (ch, st) = if col >= tl && col < tl + tw { ('▬', thumb) } else { ('─', track) };
        if let Some(c) = buf.cell_mut((x + col, y)) { c.set_char(ch); c.set_style(st); }
    }
}

fn draw_vscrollbar(buf: &mut Buffer, x: u16, y: u16, height: u16, total: usize, offset: usize) {
    if height == 0 { return; }
    let track = Style::default().fg(Color::DarkGray);
    let thumb = Style::default().fg(Color::Gray);
    if total <= height as usize {
        for row in 0..height {
            if let Some(c) = buf.cell_mut((x, y + row)) { c.set_char('│'); c.set_style(track); }
        }
        return;
    }
    let th = ((height as f64 * height as f64) / total as f64).ceil() as u16;
    let th = th.max(1).min(height);
    let range = total - height as usize;
    let tt = ((offset as f64 / range as f64) * (height - th) as f64).round() as u16;
    for row in 0..height {
        let (ch, st) = if row >= tt && row < tt + th { ('█', thumb) } else { ('│', track) };
        if let Some(c) = buf.cell_mut((x, y + row)) { c.set_char(ch); c.set_style(st); }
    }
}

// ── Content measurement ───────────────────────────────────────────────────────

fn text_line_count(s: &str) -> usize {
    if s.is_empty() { 1 } else { s.chars().filter(|&c| c == '\n').count() + 1 }
}
fn text_max_line_width(s: &str) -> usize {
    s.lines().map(|l| l.chars().count()).max().unwrap_or(0)
}
fn wrapped_lines(s: &str, width: u16) -> u16 {
    if width == 0 || s.is_empty() { return 1; }
    let mut rows = 1u16;
    let mut col = 0u16;
    for ch in s.chars() {
        if ch == '\n' { rows += 1; col = 0; }
        else { col += 1; if col >= width { rows += 1; col = 0; } }
    }
    rows
}

// ── Border helper ─────────────────────────────────────────────────────────────

fn focused_block<'a>(title: &'a str, focused: bool) -> Block<'a> {
    let style = if focused { Style::default().fg(Color::Yellow) } else { Style::default() };
    Block::bordered().title(title).border_style(style)
}

// ── Widget renderers ──────────────────────────────────────────────────────────

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
        .block(focused_block("Pattern  (Esc quit · Tab cycle)", focused))
        .wrap(Wrap { trim: false })
        .render(area, f.buffer_mut());

    if focused {
        let iw = area.width.saturating_sub(2);
        let ci = app.input[..app.cursor_pos].chars().count() as u16;
        let (cr, cc) = if iw > 0 { (ci / iw, ci % iw) } else { (0, ci) };
        f.set_cursor_position((area.x + 1 + cc, area.y + 1 + cr));
    }
}

fn render_text_to_match(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::TextToMatch;
    let block = focused_block("Text to match", focused);
    let inner = block.inner(area);
    block.render(area, f.buffer_mut());

    if inner.width < 2 || inner.height < 2 { return; }
    let cw = (inner.width - 1) as usize;
    let ch = (inner.height - 1) as usize;

    let total_rows = text_line_count(&app.text);
    let total_cols = text_max_line_width(&app.text);
    app.text_vscroll = app.text_vscroll.min(total_rows.saturating_sub(ch));
    app.text_hscroll = app.text_hscroll.min(total_cols.saturating_sub(cw));

    Paragraph::new(app.text.as_str())
        .scroll((app.text_vscroll as u16, app.text_hscroll as u16))
        .render(Rect { x: inner.x, y: inner.y, width: cw as u16, height: ch as u16 },
                f.buffer_mut());

    draw_vscrollbar(f.buffer_mut(), inner.x + cw as u16, inner.y,
                    ch as u16, total_rows.max(ch), app.text_vscroll);
    draw_hscrollbar(f.buffer_mut(), inner.x, inner.y + ch as u16,
                    cw as u16, total_cols.max(cw), app.text_hscroll);

    if focused {
        let before = &app.text[..app.text_cursor];
        let cur_row = before.chars().filter(|&c| c == '\n').count();
        let last_nl = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let cur_col = before[last_nl..].chars().count();
        if cur_row >= app.text_vscroll && cur_col >= app.text_hscroll {
            let vr = (cur_row - app.text_vscroll) as u16;
            let vc = (cur_col - app.text_hscroll) as u16;
            if vr < ch as u16 && vc < cw as u16 {
                f.set_cursor_position((inner.x + vc, inner.y + vr));
            }
        }
    }
}

fn render_tree_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::DescTree;
    let block = focused_block("Description", focused);
    let inner = block.inner(area);
    block.render(area, f.buffer_mut());
    app.tree.render(inner, f.buffer_mut(), focused);
}

// ── Main UI ───────────────────────────────────────────────────────────────────

fn ui(f: &mut Frame, app: &mut App) {
    let sides = Layout::horizontal([
        Constraint::Percentage(40),
        Constraint::Percentage(60),
    ]).split(f.area());

    let iw = sides[0].width.saturating_sub(2);
    let ih = (wrapped_lines(&app.input, iw) + 2).max(3);
    let left = Layout::vertical([Constraint::Length(ih), Constraint::Min(0)])
        .split(sides[0]);

    render_pattern_input(f, app, left[0]);
    render_text_to_match(f, app, left[1]);
    render_tree_panel(f, app, sides[1]);
}

// ── Event loop ────────────────────────────────────────────────────────────────

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

        if !event::poll(std::time::Duration::from_millis(50))? { continue; }
        let Event::Key(key) = event::read()? else { continue; };
        if key.kind != KeyEventKind::Press { continue; }

        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) | (KeyModifiers::CONTROL, KeyCode::Char('c')) => break,
            (_, KeyCode::Tab) => { app.focus = app.focus.next(); continue; }
            _ => {}
        }

        match app.focus {
            Focus::PatternInput => match (key.modifiers, key.code) {
                (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => app.input_insert(c),
                (_, KeyCode::Backspace) => app.input_backspace(),
                (_, KeyCode::Left)  => app.input_left(),
                (_, KeyCode::Right) => app.input_right(),
                (_, KeyCode::Home)  => app.cursor_pos = 0,
                (_, KeyCode::End)   => app.cursor_pos = app.input.len(),
                _ => {}
            },

            Focus::TextToMatch => {
                let mv = text_line_count(&app.text).saturating_sub(1);
                let mh = text_max_line_width(&app.text).saturating_sub(1);
                match (key.modifiers, key.code) {
                    (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => app.text_insert(c),
                    (_, KeyCode::Enter)     => app.text_newline(),
                    (_, KeyCode::Backspace) => app.text_backspace(),
                    (_, KeyCode::Up)    => app.text_vscroll = app.text_vscroll.saturating_sub(1),
                    (_, KeyCode::Down)  => { if app.text_vscroll < mv { app.text_vscroll += 1; } }
                    (_, KeyCode::Left)  => app.text_hscroll = app.text_hscroll.saturating_sub(1),
                    (_, KeyCode::Right) => { if app.text_hscroll < mh { app.text_hscroll += 1; } }
                    _ => {}
                }
            },

            Focus::DescTree => match (key.modifiers, key.code) {
                (_, KeyCode::Char('j')) => app.tree.select_down(),
                (_, KeyCode::Char('k')) => app.tree.select_up(),
                (_, KeyCode::Char('h')) => app.tree.select_left(),
                (_, KeyCode::Char('l')) => app.tree.select_right(),
                (_, KeyCode::Down) => app.tree.scroll_down(),
                (_, KeyCode::Up)   => app.tree.scroll_up(),
                _ => {}
            },
        }
    }
    Ok(())
}
