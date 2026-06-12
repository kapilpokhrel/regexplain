use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame, layout::{Constraint, Layout, Rect}, style::{Color, Style}, text::{Line, Span}, widgets::{Block, Widget}
};
use regex::bytes::Regex;

use crate::colorize::{ColorGenerator, Colorizer};
use crate::convert;
use crate::desc::{DescGenerator, DescNode, Describer};
use crate::tree::{Node, TreeWidget};
use crate::textarea::TextMatchWidget;
use crate::inputarea::InputLineWidget;

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

    inputarea: InputLineWidget,

    tree: TreeWidget,
    textmatch_widget: TextMatchWidget,

    pattern_line: Line<'static>,
    error: Option<String>,

    // Keep last parsed pattern & colors so tree selection can update the input line styling.
    last_cgen: Option<ColorGenerator>,
    last_desc: Option<crate::desc::DescNode>,
}

impl App {
    fn new() -> Self {
        Self {
            focus: Focus::PatternInput,
            inputarea: InputLineWidget::new(),
            tree: TreeWidget::new(),
            textmatch_widget: TextMatchWidget::new(),
            pattern_line: Line::default(),
            error: None,
            last_cgen: None,
            last_desc: None,
        }
    }

    fn reparse(&mut self) {
        let input = self.inputarea.pattern_str();
        if input.is_empty() {
            self.pattern_line = Line::default();
            self.error = None;
            self.tree.set_nodes(vec![]);
            self.last_cgen = None;
            return;
        }
        match convert::parse_and_convert(&input) {
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

                let re = Regex::new(&input).ok(); // we have already checked for the error
                self.textmatch_widget.update_regex(re);
                self.update_pattern_line();
            }
            Err(e) => {
                self.error = Some(e.to_string());
                self.pattern_line = Line::default();
                self.tree.set_nodes(vec![]);
                self.last_cgen = None;
                self.last_desc = None;
                self.inputarea.clear_highlight();
            }
        }
    }

    fn get_selected_span(&self) -> Option<crate::types::Span> {
        // Treat nodes with an empty desc as transparent (hoisted children), mirroring desc_to_nodes.
        fn visible_children<'a>(node: &'a crate::desc::DescNode, out: &mut Vec<&'a crate::desc::DescNode>) {
            for child in &node.nested_items {
                if child.desc.is_empty() {
                    visible_children(child, out);
                } else {
                    out.push(child);
                }
            }
        }

        if let Some(d) = &self.last_desc {
            let mut cur = d;
            for &idx in self.tree.selected_path() {
                let mut vis = Vec::new();
                visible_children(cur, &mut vis);
                if idx >= vis.len() {
                    break;
                }
                cur = vis[idx];
            }
            Some(cur.span)
        } else {
            None
        }
    }

    fn update_pattern_line(&mut self) {
        if let Some(c) = &self.last_cgen {
            let s = if let Some(span) = self.get_selected_span() && self.focus == Focus::DescTree {
                Some(span)
            } else {
                None
            };
            self.inputarea.render_input_line(c, s);
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
    spans.extend(render_slice(pattern, span.start, span.end, cgen).spans);
    spans.push(Span::raw(format!("` {}", node.desc)));

    vec![Node::new(Line::from(spans), children)]
}

fn render_slice(
    pattern: &str,
    start: usize,
    end: usize,
    cgen: &ColorGenerator,
) -> Line<'static> {
    if start >= end {
        return Line::default();
    }
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut seg = start;
    let (mut cfg, mut cbg) = cgen.char_color(start);
    for (rel, _) in pattern[start..end].char_indices().skip(1) {
        let abs = start + rel;
        let (fg, bg) = cgen.char_color(abs);
        if fg != cfg || bg != cbg {
            push_span(pattern, seg, abs, cfg, cbg, &mut spans);
            seg = abs;
            cfg = fg;
            cbg = bg;
        }
    }
    push_span(pattern, seg, end, cfg, cbg, &mut spans);
    Line::from(spans)
}

fn to_color(rgb: [f32; 3]) -> Color {
    let b = |v: f32| (v * 255.0).clamp(0.0, 255.0) as u8;
    Color::Rgb(b(rgb[0]), b(rgb[1]), b(rgb[2]))
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
    let block = focused_block("Pattern  (Esc quit · Shift+Tab cycle)", focused);
    let inner = block.inner(area);
    block.render(area, f.buffer_mut());

    if inner.height < 1 {
        return;
    }
    app.inputarea.render_inputline(f, inner);
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
    app.textmatch_widget.render_textarea(f, editor);
    app.textmatch_widget.render_status_line(f, status);
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
    let ih = (wrapped_lines(&app.inputarea.pattern_str(), iw) + 2).max(3);
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
            Focus::PatternInput => {
                if !app.inputarea.input(key) {
                    continue;
                }
                app.reparse();
            },

            Focus::TextToMatch => {
                if !app.textmatch_widget.input(key) {
                    continue;
                }
                app.textmatch_widget.eval_regex();
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
