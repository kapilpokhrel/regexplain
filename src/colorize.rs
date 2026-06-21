use crate::types::*;
use ratatui::{style::{Color, Style}, text::Span as RatatuiSpan};

const TERMINAL_BG: [f32; 3] = [0.0, 0.0, 0.0];

// At least one channel at 1.0 so brackets stay vivid on their own dark-tinted bg
const GROUP_COLOR: [f32; 3] = [0.0, 1.0, 0.65];  // bright teal
const GROUP_ALPHA: f32 = 0.2; // alpha is added for bg, but fg is solid with same color

const CLASS_BG:     [f32; 3] = [0.37, 0.69, 0.84];  // soft blue
const CLASS_ALPHA: f32 = 0.35;

const LITERAL_FG:    [f32; 3] = [1.0,  1.0,  1.0 ];  // white
const ESCAPED_FG:    [f32; 3] = [1.0,  0.6,  0.85];  // pink
const ASSERTION_FG:  [f32; 3] = [0.0,  1.0,  1.0 ];  // bright cyan
const QUANTIFIER_FG: [f32; 3] = [1.0,  0.85, 0.0 ];  // bright amber
const FLAG_FG:       [f32; 3] = [0.75, 0.5,  1.0 ];  // bright lavender
const ALT_FG:        [f32; 3] = [1.0,  0.35, 0.6 ];  // bright rose
const OPERATOR_FG:   [f32; 3] = [1.0,  0.85, 0.0 ];  // bright amber
const PERL_FG:       [f32; 3] = [1.0,  0.7,  0.15];  // bright orange
const UNICODE_FG:    [f32; 3] = [1.0,  0.6,  0.85];  // pink
const ASCII_FG:      [f32; 3] = [1.0,  0.95, 0.2 ];  // bright yellow
const RANGE_FG:      [f32; 3] = [0.55, 1.0,  0.35];  // bright lime

#[derive(Debug, Clone, Copy)]
pub struct ColorSpan {
    pub span: PatternSpan,
    pub fg: Option<[f32; 3]>,  // RGB; narrowest span wins
    pub bg: Option<[f32; 4]>,  // RGBA; background is composited (added layer by layer)
}

impl ColorSpan {
    fn fg(span: PatternSpan, c: [f32; 3]) -> Self { Self { span, fg: Some(c), bg: None } }
    fn bg(span: PatternSpan, c: [f32; 4]) -> Self { Self { span, fg: None,    bg: Some(c) } }
}

#[derive(Clone)]
pub struct ColorGenerator {
    spans: Vec<ColorSpan>,
}

impl ColorGenerator {
    pub fn new() -> Self {
        Self { spans: Vec::new() }
    }

    fn add_span(&mut self, span: ColorSpan) {
        self.spans.push(span);
    }

    pub fn sort_spans(&mut self) {
        // sort spans by widest first
        self.spans.sort_by_key(|s| std::cmp::Reverse(s.span.end - s.span.start));
    }

    /// assume already sorted by widest span first
    pub fn char_color(&self, i: usize) -> (Option<[f32; 3]>, Option<[f32; 3]>) {
        fn over(dst: [f32; 3], src: [f32; 4]) -> [f32; 3] {
            let a = src[3];
            [src[0]*a + dst[0]*(1.0-a), src[1]*a + dst[1]*(1.0-a), src[2]*a + dst[2]*(1.0-a)]
        }

        let mut bg_color = TERMINAL_BG;
        let mut has_bg = false;
        for s in &self.spans {
            if s.span.start <= i && i < s.span.end && let Some(rbga) = s.bg {
                bg_color = over(bg_color, rbga);
                has_bg = true;
            }
        }
        let bg = if has_bg { Some(bg_color) } else { None };
        let mut fg = None;
        for s in self.spans.iter().rev() { //narrowest first
            if s.span.start <= i && i < s.span.end && s.fg.is_some() {
                fg = s.fg;
                break; // first match is the narrowest
            }
        }
        (fg, bg)
    }

    pub fn ratatui_colored_slice(
        &self,
        pattern: &str,
        start: usize,
        end: usize,
        fg_bright_factor: f32,
    ) -> Vec<RatatuiSpan<'static>> {
        if start >= end {
            return Vec::new();
        }
        let mut spans: Vec<RatatuiSpan<'static>> = Vec::new();
        for idx in start..end {
            let (fg, bg) = self.char_color(idx);
            let fg_b = brighten_fg(fg, fg_bright_factor);

            let mut style = Style::default();
            if let Some(f) = fg_b {
                style = style.fg(to_color(f));
            }
            if let Some(b) = bg {
                style = style.bg(to_color(b));
            }
            spans.push(RatatuiSpan::styled(pattern[idx..idx+1].to_string(), style));
        }
        spans
    }
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

fn to_color(rgb: [f32; 3]) -> Color {
    let b = |v: f32| (v * 255.0).clamp(0.0, 255.0) as u8;
    Color::Rgb(b(rgb[0]), b(rgb[1]), b(rgb[2]))
}

pub trait Colorizer<T> {
    fn colorize(&mut self, target: T);
}

impl Colorizer<&RegExplainForm> for ColorGenerator {
    fn colorize(&mut self, form: &RegExplainForm) {
        self.colorize(&form.root);
        self.sort_spans();
    }
}

impl Colorizer<&RegExplainSimplifiedNode> for ColorGenerator {
    fn colorize(&mut self, node: &RegExplainSimplifiedNode) {
        match node {
            RegExplainSimplifiedNode::Literal(l) => self.colorize(l),
            RegExplainSimplifiedNode::Assertion(a) => self.add_span(ColorSpan::fg(a.span, ASSERTION_FG)),
            RegExplainSimplifiedNode::Flags(f)     => self.add_span(ColorSpan::fg(f.span, FLAG_FG)),
            RegExplainSimplifiedNode::Class(c) => self.colorize(c),
            RegExplainSimplifiedNode::Group(g) => self.colorize(g),
            RegExplainSimplifiedNode::Repeat(r) => self.colorize(r),
            RegExplainSimplifiedNode::Alt { alts, .. } => {
                // i wanted to color the pipe symbol sperately
                for w in alts.windows(2) {
                    let pipe = PatternSpan { start: w[0].span().end, end: w[1].span().start };
                    self.add_span(ColorSpan::fg(pipe, ALT_FG));
                }
                for alt in alts { self.colorize(alt); }
            }
            RegExplainSimplifiedNode::Concat { nodes, .. } => {
                for node in nodes { self.colorize(node); }
            }
        }
    }
}

impl Colorizer<&GroupNode> for ColorGenerator {
    fn colorize(&mut self, target: &GroupNode) {
        let c = GROUP_COLOR;
        let inner = target.inner.span();

        self.add_span(ColorSpan::bg(target.span, [c[0], c[1], c[2], GROUP_ALPHA]));

        // brackets around the group's inner content get a bright fg on top of that bg
        let prefix = PatternSpan { start: target.span.start, end: inner.start };
        let suffix = PatternSpan { start: inner.end,    end: target.span.end  };
        if prefix.start < prefix.end { self.add_span(ColorSpan::fg(prefix, c)); }
        if suffix.start < suffix.end { self.add_span(ColorSpan::fg(suffix, c)); }

        self.colorize(&*target.inner);
    }
}
impl Colorizer<&RepeatNode> for ColorGenerator {
    fn colorize(&mut self, target: &RepeatNode) {
        let op = PatternSpan { start: target.inner.span().end, end: target.span.end };
        self.add_span(ColorSpan::fg(op, QUANTIFIER_FG));
        self.colorize(&*target.inner);
    }
}

impl Colorizer<&LiteralNode> for ColorGenerator {
    fn colorize(&mut self, target: &LiteralNode) {
        let fg = match target.ch { LiteralChar::Verbatim(_) => LITERAL_FG, _ => ESCAPED_FG };
        self.add_span(ColorSpan::fg(target.span, fg));
    }
}

impl Colorizer<&ClassNode> for ColorGenerator {
    fn colorize(&mut self, target: &ClassNode) {
        if let Some(bg) = class_kind_bg(&target.kind) {
            self.add_span(ColorSpan::bg(target.span, bg));
        }
        if let Some(fg) = class_kind_fg(&target.kind) {
            self.add_span(ColorSpan::fg(target.span, fg));
        }
        self.colorize(&target.kind);
    }
}

impl Colorizer<&ClassKind> for ColorGenerator {
    fn colorize(&mut self, target: &ClassKind) {
        match target {
            ClassKind::BracketedOp { lhs, rhs, .. } => {
                let op = PatternSpan { start: lhs.span.end, end: rhs.span.start };
                self.add_span(ColorSpan::fg(op, OPERATOR_FG));
                self.colorize(&*lhs.kind);
                self.colorize(&*rhs.kind);
            }
            ClassKind::Bracketed(items) => {
                for item in items {
                    match item {
                        ClassItem::Literal(l) => self.colorize(l),
                        ClassItem::Range { span, .. } => self.add_span(ColorSpan::fg(*span, RANGE_FG)),
                        ClassItem::Class(c) => self.colorize(c),
                    }
                }
            }
            _ => {}
        }
    }
}



fn class_kind_fg(kind: &ClassKind) -> Option<[f32; 3]> {
    match kind {
        ClassKind::PerlDigit | ClassKind::PerlSpace | ClassKind::PerlWord => Some(PERL_FG),
        ClassKind::Unicode(_) => Some(UNICODE_FG),
        ClassKind::AsciiAlnum  | ClassKind::AsciiAlpha | ClassKind::AsciiAscii |
        ClassKind::AsciiBlank  | ClassKind::AsciiCntrl | ClassKind::AsciiDigit |
        ClassKind::AsciiGraph  | ClassKind::AsciiLower | ClassKind::AsciiPrint |
        ClassKind::AsciiPunct  | ClassKind::AsciiSpace | ClassKind::AsciiUpper |
        ClassKind::AsciiWord   | ClassKind::AsciiXdigit => Some(ASCII_FG),
        _ => None,
    }
}

fn class_kind_bg(kind: &ClassKind) -> Option<[f32; 4]> {
    match kind {
        ClassKind::Bracketed(_)
        | ClassKind::BracketedOp { op: _, lhs: _, rhs: _ }
        => Some([CLASS_BG[0], CLASS_BG[1], CLASS_BG[2], CLASS_ALPHA]),
        _ => None,
    }
}
