use std::iter::Peekable;

use regex_syntax::ast::parse::Parser;
use regex_syntax::ast::{self, Ast};

use crate::types::*;

pub fn parse_and_convert(pattern: &str) -> Result<RegExplainForm, ast::Error> {
    let ast = Parser::new().parse(pattern)?;
    Ok(RegExplainForm {
        pattern: pattern.to_string(),
        root: RegExplainSimplifiedNode::from(&ast),
    })
}

fn span(s: &ast::Span) -> Span {
    Span {
        start: s.start.offset,
        end: s.end.offset,
    }
}

fn flags_items(flags: &ast::Flags) -> Vec<FlagItem> {
    let mut negated = false;
    flags
        .items
        .iter()
        .filter_map(|item| match &item.kind {
            ast::FlagsItemKind::Negation => {
                negated = true;
                None
            }
            ast::FlagsItemKind::Flag(f) => Some(FlagItem {
                span: span(&item.span),
                negated,
                kind: FlagKind::from(f),
            }),
        })
        .collect()
}

fn collect_items(item: &ast::ClassSetItem) -> Vec<ClassItem> {
    match item {
        ast::ClassSetItem::Empty(_) => vec![],
        ast::ClassSetItem::Union(u) => u.items.iter().flat_map(collect_items).collect(),
        other => vec![ClassItem::from(other)],
    }
}

fn merge_verbatim<I>(first: &mut RegExplainSimplifiedNode, items: &mut Peekable<I>)
where
    I: Iterator<Item = RegExplainSimplifiedNode>,
{
    if let RegExplainSimplifiedNode::Literal(LiteralNode {
        ch: LiteralChar::Verbatim(curr_ch),
        span: curr_span,
    }) = first
    {
        while let Some(RegExplainSimplifiedNode::Literal(LiteralNode {
            ch: LiteralChar::Verbatim(next_ch),
            span: s,
        })) = items.peek()
        {
            curr_span.end = s.end;
            curr_ch.push_str(next_ch);
            items.next();
        }
    }
}

impl From<&Ast> for RegExplainSimplifiedNode {
    fn from(ast: &Ast) -> Self {
        match ast {
            Ast::Empty(_) => Self::Concat { span: Span { start: 0, end: 0 }, nodes: vec![] },
            Ast::Dot(s) => Self::Class(ClassNode {
                span: span(s),
                negated: false,
                kind: ClassKind::Dot,
            }),
            Ast::Flags(f) => Self::Flags(FlagNode::from(f.as_ref())),
            Ast::Literal(l) => Self::Literal(LiteralNode::from(l.as_ref())),
            Ast::Assertion(a) => Self::Assertion(AssertionNode::from(a.as_ref())),
            Ast::Repetition(r) => Self::Repeat(RepeatNode::from(r.as_ref())),
            Ast::Group(g) => Self::Group(GroupNode::from(g.as_ref())),
            Ast::ClassBracketed(c) => Self::Class(ClassNode::from(c.as_ref())),
            Ast::ClassUnicode(c) => Self::Class(ClassNode::from(c.as_ref())),
            Ast::ClassPerl(c) => Self::Class(ClassNode::from(c.as_ref())),
            Ast::Alternation(a) => Self::Alt {
                span: span(&a.span),
                alts: a.asts.iter().map(Self::from).collect(),
            },
            Ast::Concat(c) => {
                let mut nodes: Vec<RegExplainSimplifiedNode> = Vec::new();
                let mut items = c.asts.iter().map(RegExplainSimplifiedNode::from).peekable();
                while let Some(mut node) = items.next() {
                    if let RegExplainSimplifiedNode::Literal(LiteralNode {
                        ch: LiteralChar::Verbatim(_),
                        ..
                    }) = node
                    {
                        merge_verbatim(&mut node, &mut items);
                    }
                    nodes.push(node)
                }
                Self::Concat { span: span(&c.span), nodes }
            }
        }
    }
}

impl From<&ast::Literal> for LiteralNode {
    fn from(l: &ast::Literal) -> Self {
        let ch = if let ast::LiteralKind::Special(s) = &l.kind {
            LiteralChar::Special(match s {
                ast::SpecialLiteralKind::Bell => SpecialChar::Bell,
                ast::SpecialLiteralKind::FormFeed => SpecialChar::FormFeed,
                ast::SpecialLiteralKind::Tab => SpecialChar::Tab,
                ast::SpecialLiteralKind::LineFeed => SpecialChar::LineFeed,
                ast::SpecialLiteralKind::CarriageReturn => SpecialChar::CarriageReturn,
                ast::SpecialLiteralKind::VerticalTab => SpecialChar::VerticalTab,
                ast::SpecialLiteralKind::Space => SpecialChar::Space,
            })
        } else {
            let c = l.c.to_string();
            match &l.kind {
                ast::LiteralKind::Verbatim
                | ast::LiteralKind::Meta
                | ast::LiteralKind::Superfluous => LiteralChar::Verbatim(c),
                ast::LiteralKind::Octal => LiteralChar::Octal(c),
                _ => LiteralChar::Hex(c),
            }
        };
        LiteralNode {
            span: span(&l.span),
            ch,
        }
    }
}

impl From<&ast::Assertion> for AssertionNode {
    fn from(a: &ast::Assertion) -> Self {
        AssertionNode {
            span: span(&a.span),
            kind: AssertionKind::from(&a.kind),
        }
    }
}

impl From<&ast::AssertionKind> for AssertionKind {
    fn from(k: &ast::AssertionKind) -> Self {
        match k {
            ast::AssertionKind::StartLine => Self::StartLine,
            ast::AssertionKind::EndLine => Self::EndLine,
            ast::AssertionKind::StartText => Self::StartText,
            ast::AssertionKind::EndText => Self::EndText,
            ast::AssertionKind::WordBoundary => Self::WordBoundary,
            ast::AssertionKind::NotWordBoundary => Self::NotWordBoundary,
            ast::AssertionKind::WordBoundaryStart | ast::AssertionKind::WordBoundaryStartAngle => {
                Self::WordBoundaryStart
            }
            ast::AssertionKind::WordBoundaryEnd | ast::AssertionKind::WordBoundaryEndAngle => {
                Self::WordBoundaryEnd
            }
            ast::AssertionKind::WordBoundaryStartHalf => Self::WordBoundaryStartHalf,
            ast::AssertionKind::WordBoundaryEndHalf => Self::WordBoundaryEndHalf,
        }
    }
}

impl From<&ast::SetFlags> for FlagNode {
    fn from(f: &ast::SetFlags) -> Self {
        FlagNode {
            span: span(&f.span),
            items: flags_items(&f.flags),
        }
    }
}

impl From<&ast::Flag> for FlagKind {
    fn from(f: &ast::Flag) -> Self {
        match f {
            ast::Flag::CaseInsensitive => Self::CaseInsensitive,
            ast::Flag::MultiLine => Self::MultiLine,
            ast::Flag::DotMatchesNewLine => Self::DotMatchesNewLine,
            ast::Flag::SwapGreed => Self::SwapGreed,
            ast::Flag::Unicode => Self::Unicode,
            ast::Flag::CRLF => Self::Crlf,
            ast::Flag::IgnoreWhitespace => Self::IgnoreWhitespace,
        }
    }
}

impl From<&ast::Repetition> for RepeatNode {
    fn from(r: &ast::Repetition) -> Self {
        let (min, max) = match r.op.kind {
            ast::RepetitionKind::ZeroOrOne => (0, Some(1)),
            ast::RepetitionKind::ZeroOrMore => (0, None),
            ast::RepetitionKind::OneOrMore => (1, None),
            ast::RepetitionKind::Range(ref range) => match *range {
                ast::RepetitionRange::Exactly(n) => (n, Some(n)),
                ast::RepetitionRange::AtLeast(n) => (n, None),
                ast::RepetitionRange::Bounded(lo, hi) => (lo, Some(hi)),
            },
        };
        RepeatNode {
            span: span(&r.span),
            greedy: r.greedy,
            min,
            max,
            inner: Box::new(r.ast.as_ref().into()),
        }
    }
}

impl From<&ast::Group> for GroupNode {
    fn from(g: &ast::Group) -> Self {
        GroupNode {
            span: span(&g.span),
            kind: match &g.kind {
                ast::GroupKind::CaptureIndex(idx) => GroupKind::Capture {
                    index: *idx,
                    name: None,
                },
                ast::GroupKind::CaptureName { name, .. } => GroupKind::Capture {
                    index: name.index,
                    name: Some(name.name.clone()),
                },
                ast::GroupKind::NonCapturing(flags) => GroupKind::NonCapturing(flags_items(flags)),
            },
            inner: Box::new(g.ast.as_ref().into()),
        }
    }
}

impl From<&ast::ClassBracketed> for ClassNode {
    fn from(c: &ast::ClassBracketed) -> Self {
        ClassNode {
            span: span(&c.span),
            negated: c.negated,
            kind: ClassKind::from(&c.kind),
        }
    }
}

impl From<&ast::ClassUnicode> for ClassNode {
    fn from(c: &ast::ClassUnicode) -> Self {
        ClassNode {
            span: span(&c.span),
            negated: c.negated,
            kind: ClassKind::Unicode(UnicodeClassKind::from(&c.kind)),
        }
    }
}

impl From<&ast::ClassPerl> for ClassNode {
    fn from(c: &ast::ClassPerl) -> Self {
        ClassNode {
            span: span(&c.span),
            negated: c.negated,
            kind: ClassKind::from(&c.kind),
        }
    }
}

impl From<&ast::ClassAscii> for ClassNode {
    fn from(c: &ast::ClassAscii) -> Self {
        ClassNode {
            span: span(&c.span),
            negated: c.negated,
            kind: ClassKind::from(&c.kind),
        }
    }
}

impl From<&ast::ClassSet> for ClassKind {
    fn from(s: &ast::ClassSet) -> Self {
        match s {
            ast::ClassSet::Item(item) => ClassKind::Bracketed(collect_items(item)),
            ast::ClassSet::BinaryOp(op) => ClassKind::BracketedOp {
                op: ClassBinaryOp::from(op.kind),
                lhs: ClassOperand {
                    span: Span::from(op.lhs.as_ref()),
                    kind: Box::new(ClassKind::from(op.lhs.as_ref())),
                },
                rhs: ClassOperand {
                    span: Span::from(op.rhs.as_ref()),
                    kind: Box::new(ClassKind::from(op.rhs.as_ref())),
                },
            },
        }
    }
}

impl From<&ast::ClassSet> for Span {
    fn from(s : &ast::ClassSet) -> Self {
        match s {
            ast::ClassSet::Item(item) => {
                let items_spans: Vec<Span> = collect_items(item).into_iter().map(|x| {
                    match x {
                        ClassItem::Literal(l) => l.span,
                        ClassItem::Range{span: s, ..} => s,
                        ClassItem::Class(c) => c.span
                    }
                }).collect();
                if items_spans.is_empty() {
                    Span{start: 0, end: 0}
                } else {
                    Span{
                        start: items_spans.first().unwrap().start,
                        end: items_spans.last().unwrap().end
                    }
                }
            }
            ast::ClassSet::BinaryOp(op) => span(&op.span)
        }
    }
}

impl From<ast::ClassSetBinaryOpKind> for ClassBinaryOp {
    fn from(k: ast::ClassSetBinaryOpKind) -> Self {
        match k {
            ast::ClassSetBinaryOpKind::Intersection => Self::Intersection,
            ast::ClassSetBinaryOpKind::Difference => Self::Difference,
            ast::ClassSetBinaryOpKind::SymmetricDifference => Self::SymmetricDifference,
        }
    }
}

impl From<&ast::ClassSetItem> for ClassItem {
    fn from(item: &ast::ClassSetItem) -> Self {
        match item {
            ast::ClassSetItem::Empty(_) | ast::ClassSetItem::Union(_) => {
                unreachable!("handled by collect_items")
            }
            ast::ClassSetItem::Literal(l) => ClassItem::Literal(LiteralNode::from(l)),
            ast::ClassSetItem::Range(r) => ClassItem::Range {
                span: span(&r.span),
                start: r.start.c,
                end: r.end.c,
            },
            ast::ClassSetItem::Unicode(c) => ClassItem::Class(ClassNode::from(c)),
            ast::ClassSetItem::Perl(c) => ClassItem::Class(ClassNode::from(c)),
            ast::ClassSetItem::Ascii(c) => ClassItem::Class(ClassNode::from(c)),
            ast::ClassSetItem::Bracketed(b) => ClassItem::Class(ClassNode::from(b.as_ref())),
        }
    }
}

impl From<&ast::ClassUnicodeKind> for UnicodeClassKind {
    fn from(k: &ast::ClassUnicodeKind) -> Self {
        match k {
            ast::ClassUnicodeKind::OneLetter(c) => Self::Named(c.to_string()),
            ast::ClassUnicodeKind::Named(s) => Self::Named(s.clone()),
            ast::ClassUnicodeKind::NamedValue { op, name, value } => Self::NamedValue {
                negated: matches!(op, ast::ClassUnicodeOpKind::NotEqual),
                name: name.clone(),
                value: value.clone(),
            },
        }
    }
}

impl From<&ast::ClassPerlKind> for ClassKind {
    fn from(k: &ast::ClassPerlKind) -> Self {
        match k {
            ast::ClassPerlKind::Digit => Self::PerlDigit,
            ast::ClassPerlKind::Space => Self::PerlSpace,
            ast::ClassPerlKind::Word => Self::PerlWord,
        }
    }
}

impl From<&ast::ClassAsciiKind> for ClassKind {
    fn from(k: &ast::ClassAsciiKind) -> Self {
        match k {
            ast::ClassAsciiKind::Alnum => Self::AsciiAlnum,
            ast::ClassAsciiKind::Alpha => Self::AsciiAlpha,
            ast::ClassAsciiKind::Ascii => Self::AsciiAscii,
            ast::ClassAsciiKind::Blank => Self::AsciiBlank,
            ast::ClassAsciiKind::Cntrl => Self::AsciiCntrl,
            ast::ClassAsciiKind::Digit => Self::AsciiDigit,
            ast::ClassAsciiKind::Graph => Self::AsciiGraph,
            ast::ClassAsciiKind::Lower => Self::AsciiLower,
            ast::ClassAsciiKind::Print => Self::AsciiPrint,
            ast::ClassAsciiKind::Punct => Self::AsciiPunct,
            ast::ClassAsciiKind::Space => Self::AsciiSpace,
            ast::ClassAsciiKind::Upper => Self::AsciiUpper,
            ast::ClassAsciiKind::Word => Self::AsciiWord,
            ast::ClassAsciiKind::Xdigit => Self::AsciiXdigit,
        }
    }
}
