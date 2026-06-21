use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::clipboard;
use crossterm::execute;
use ratatui::{
    Frame, layout::{Constraint, Layout, Rect}, style::{Color, Style}, widgets::{Widget, Block}
};
use regex::bytes::Regex;

use crate::colorize::{ColorGenerator, Colorizer};
use crate::convert;
use crate::desc::{DescGenerator, Describer};
use crate::tui::desctree::DescTreeWidget;
use crate::tui::match_editor::MatchEditorWidget;
use crate::tui::inputarea::InputLineWidget;

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
    desctree_widget: DescTreeWidget,
    match_editor: MatchEditorWidget,

    error: Option<String>,

    color_generator: Option<ColorGenerator>
}

impl App {
    fn new() -> Self {
        Self {
            focus: Focus::PatternInput,
            inputarea: InputLineWidget::new(),
            desctree_widget: DescTreeWidget::new(),
            match_editor: MatchEditorWidget::new(),
            error: None,

            color_generator: None,
        }
    }

    fn set_pattern(&mut self, p: impl Into<String>) {
        self.inputarea.set_pattern(p);
    }

    fn set_match_text(&mut self, t: impl Into<String>) {
        self.match_editor.set_match_text(t);
    }

    fn reparse(&mut self) {
        let input = self.inputarea.pattern_str();
        if input.is_empty() {
            self.error = None;
            self.color_generator = None;
            return;
        }
        match convert::parse_and_convert(&input) {
            Ok(form) => {
                self.error = None;
                let mut cgen = ColorGenerator::new();
                cgen.colorize(&form);

                self.desctree_widget = DescTreeWidget::from_regex_description_tree(
                    &DescGenerator::new().describe(form.root),
                    &form.pattern,
                    &cgen
                );

                self.color_generator = Some(cgen);
                let re = Regex::new(&input).ok(); // we have already checked for the error
                self.match_editor.update_regex(re);
                self.update_input_pattern();
            }
            Err(e) => {
                self.error = Some(e.to_string());
                self.color_generator = None;
                self.inputarea.clear_highlight();
            }
        }
    }

    fn update_input_pattern(&mut self) {
        if let Some(c) = &self.color_generator {
            let span = if self.focus == Focus::DescTree {
                self.desctree_widget.get_selected_span()
            } else {
                None
            };
            self.inputarea.render_input_line(c, span);
        }
    }
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
    f.render_widget(&app.inputarea, inner);
}

fn render_text_to_match(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::TextToMatch;
    let block = focused_block("Text to match", focused);
    let inner = block.inner(area);
    block.render(area, f.buffer_mut());

    if inner.width < 2 || inner.height < 2 {
        return;
    }
    f.render_widget(&app.match_editor, inner);
}

fn render_tree_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::DescTree;
    let (sel, total) = app.desctree_widget.selected_index_total();
    let title = match sel {
        Some(i) => format!("Description: {}/{}", i, total),
        None => "Description".to_string(),
    };
    let style = if focused { Style::default().fg(Color::Yellow) } else { Style::default() };
    let block = Block::bordered().title(title).border_style(style);
    let inner = block.inner(area);
    block.render(area, f.buffer_mut());

    app.desctree_widget.set_accent_color(if focused {
        Color::Yellow
    } else {
        Color::DarkGray
    });
    f.render_widget(&mut app.desctree_widget, inner);
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

pub fn run(pattern: impl Into<String>, text_to_match: impl Into<String>) -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, pattern, text_to_match);
    ratatui::restore();
    result
}

fn run_app(terminal: &mut ratatui::DefaultTerminal, pattern: impl Into<String>, text_to_match: impl Into<String>) -> io::Result<()> {
    let mut app = App::new();
    app.set_pattern(pattern);
    app.set_match_text(text_to_match);
    app.reparse();
    app.match_editor.eval_regex();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

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
                app.update_input_pattern();
                continue;
            }
            _ => {}
        }

        match app.focus {
            Focus::PatternInput => {
                if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('y') {
                    let _ = execute!(std::io::stdout(), clipboard::CopyToClipboard::to_clipboard_from(
                        app.inputarea.pattern_str()
                    ));
                    continue;
                }
                if key.code == KeyCode::Enter {
                    continue;
                }
                if !app.inputarea.input(key) {
                    continue;
                }
                app.reparse();
                app.match_editor.eval_regex();
            },

            Focus::TextToMatch => {
                if !app.match_editor.input(key) {
                    continue;
                }
                app.match_editor.eval_regex();
            }

            Focus::DescTree => match (key.modifiers, key.code) {
                (KeyModifiers::CONTROL, KeyCode::Char('n')) => app.desctree_widget.scroll_down(),
                (KeyModifiers::CONTROL, KeyCode::Char('j')) => app.desctree_widget.scroll_up(),
                (_, KeyCode::Char('j') | KeyCode::Down) => {
                    app.desctree_widget.select_down();
                    app.update_input_pattern();
                }
                (_, KeyCode::Char('k') | KeyCode::Up) => {
                    app.desctree_widget.select_up();
                    app.update_input_pattern();
                }
                (_, KeyCode::Char('h') | KeyCode::Left) => {
                    app.desctree_widget.select_left();
                    app.update_input_pattern();
                }
                (_, KeyCode::Char('l') | KeyCode::Right) => {
                    app.desctree_widget.select_right();
                    app.update_input_pattern();
                }
                _ => {}
            },
        }
    }
    Ok(())
}
