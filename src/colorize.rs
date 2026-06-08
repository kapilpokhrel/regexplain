use crate::types::*;

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
    pub span: Span,
    pub fg: Option<[f32; 3]>,  // RGB; narrowest span wins
    pub bg: Option<[f32; 4]>,  // RGBA; background is composited (added layer by layer)
}

impl ColorSpan {
    fn fg(span: Span, c: [f32; 3]) -> Self { Self { span, fg: Some(c), bg: None } }
    fn bg(span: Span, c: [f32; 4]) -> Self { Self { span, fg: None,    bg: Some(c) } }
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

    pub fn char_color(&self, i: usize) -> (Option<[f32; 3]>, Option<[f32; 3]>) {
        fn over(dst: [f32; 3], src: [f32; 4]) -> [f32; 3] {
            let a = src[3];
            [src[0]*a + dst[0]*(1.0-a), src[1]*a + dst[1]*(1.0-a), src[2]*a + dst[2]*(1.0-a)]
        }

        // Sort bg layers widest-first (outermost = bottom) then composite
        let mut layers: Vec<(usize, [f32; 4])> = self.spans.iter()
            .filter(|s| s.span.start <= i && i < s.span.end && s.bg.is_some())
            .map(|s| (s.span.end - s.span.start, s.bg.unwrap()))
            .collect();

        layers.sort_by_key(|&(width, _)| width);
        let bg = if layers.is_empty() { None } else {
            Some(layers.iter().fold(TERMINAL_BG, |acc, &(_, rgba)| over(acc, rgba)))
        };

        // Narrowest fg span wins
        let fg = self.spans.iter()
            .filter(|s| s.span.start <= i && i < s.span.end && s.fg.is_some())
            .min_by_key(|s| s.span.end - s.span.start)
            .and_then(|s| s.fg);

        (fg, bg)
    }
}

pub trait Colorizer<T> {
    fn colorize(&mut self, target: T);
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
                    let pipe = Span { start: w[0].span().end, end: w[1].span().start };
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
        let prefix = Span { start: target.span.start, end: inner.start };
        let suffix = Span { start: inner.end,    end: target.span.end  };
        if prefix.start < prefix.end { self.add_span(ColorSpan::fg(prefix, c)); }
        if suffix.start < suffix.end { self.add_span(ColorSpan::fg(suffix, c)); }

        self.colorize(&*target.inner);
    }
}
impl Colorizer<&RepeatNode> for ColorGenerator {
    fn colorize(&mut self, target: &RepeatNode) {
        let op = Span { start: target.inner.span().end, end: target.span.end };
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
                let op = Span { start: lhs.span.end, end: rhs.span.start };
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
