use crate::types::*;

pub struct DescNode {
    pub desc: String,
    pub nested_items: Vec<DescNode>,
}

impl DescNode {
    fn leaf(desc: impl Into<String>) -> Self {
        DescNode { desc: desc.into(), nested_items: vec![] }
    }

    pub fn print(&self, indent: usize) {
        if !self.desc.is_empty() {
            println!("{}{}", "  ".repeat(indent), self.desc);
        }
        let child_indent = if self.desc.is_empty() { indent } else { indent + 1 };
        for child in &self.nested_items {
            child.print(child_indent);
        }
    }
}

impl From<RegExplainSimplifiedNode> for DescNode {
    fn from(value: RegExplainSimplifiedNode) -> Self {
        match value {
            RegExplainSimplifiedNode::Flags(f)     => f.into(),
            RegExplainSimplifiedNode::Literal(l)   => l.into(),
            RegExplainSimplifiedNode::Assertion(a) => a.into(),
            RegExplainSimplifiedNode::Class(c)     => c.into(),
            RegExplainSimplifiedNode::Group(g)     => g.into(),
            RegExplainSimplifiedNode::Repeat(r)    => r.into(),
            RegExplainSimplifiedNode::Alt(a) => DescNode {
                // i think span can be calculated from a[0].span.start to a[a.len()-1].span.end
                desc: "Selects one of the matches from the following list".into(),
                nested_items: a.into_iter().map(Self::from).collect(),
            },
            RegExplainSimplifiedNode::Concat(c) => DescNode {
                // i think span can be calculated from a[0].span.start to a[a.len()-1].span.end
                desc: String::new(),
                nested_items: c.into_iter().map(Self::from).collect(),
            },
        }
    }
}

impl From<LiteralNode> for DescNode {
    fn from(value: LiteralNode) -> Self {
        DescNode::leaf(match value.ch {
            LiteralChar::Verbatim(s) => format!("matches \"{}\" literally", s),
            LiteralChar::Octal(s)   => format!("matches character {}, (octal escaped)", s),
            LiteralChar::Hex(s)     => format!("matches character {}, (hex escaped)", s),
            LiteralChar::Special(s) => match s {
                SpecialChar::Bell          => "matches bell character (\\a) literally".into(),
                SpecialChar::FormFeed      => "matches form feed character (\\f) literally".into(),
                SpecialChar::Tab           => "matches tab character (\\t) literally".into(),
                SpecialChar::LineFeed      => "matches line feed character (\\n) literally".into(),
                SpecialChar::CarriageReturn => "matches carriage return character (\\r) literally".into(),
                SpecialChar::VerticalTab   => "matches vertical tab character (\\v) literally".into(),
                SpecialChar::Space         => "matches space character in verbose mode (\\ )".into(),
            },
        })
    }
}

impl From<AssertionNode> for DescNode {
    fn from(value: AssertionNode) -> Self {
        DescNode::leaf(match value.kind {
            AssertionKind::StartLine           => "asserts position at start of line",
            AssertionKind::EndLine             => "asserts position at the end of line",
            AssertionKind::StartText           => "asserts position at start of text",
            AssertionKind::EndText             => "asserts position at end of text",
            AssertionKind::WordBoundary        => "asserts position at word boundary",
            AssertionKind::NotWordBoundary     => "asserts position where \\b does not match",
            AssertionKind::WordBoundaryStart   => "...",
            AssertionKind::WordBoundaryEnd     => "...",
            AssertionKind::WordBoundaryStartHalf => "...",
            AssertionKind::WordBoundaryEndHalf   => "...",
        })
    }
}

impl From<FlagItem> for DescNode {
    fn from(item: FlagItem) -> Self {
        DescNode::leaf(format!(
            "{} {}",
            if item.negated { "disable" } else { "enable" },
            match item.kind {
                FlagKind::CaseInsensitive   => "case-insensitive matching",
                FlagKind::MultiLine         => "multi-line (^ and $ match line boundaries)",
                FlagKind::DotMatchesNewLine => "dot matches newline",
                FlagKind::SwapGreed         => "swap greediness",
                FlagKind::Unicode           => "Unicode mode",
                FlagKind::Crlf             => "CRLF line endings",
                FlagKind::IgnoreWhitespace  => "ignore whitespace and comments",
            }
        ))
    }
}

impl From<FlagNode> for DescNode {
    fn from(value: FlagNode) -> Self {
        DescNode {
            desc: "enable/disable the following flags:".into(),
            nested_items: value.items.into_iter().map(DescNode::from).collect(),
        }
    }
}

impl From<GroupNode> for DescNode {
    fn from(value: GroupNode) -> Self {
        let (header, mut nested) = match value.kind {
            GroupKind::Capture { index, name: None } => {
                (format!("capture group #{}", index), vec![])
            }
            GroupKind::Capture { index, name: Some(name) } => {
                (format!("capture group #{} named \"{}\"", index, name), vec![])
            }
            GroupKind::NonCapturing(flags) => (
                "non-capturing group".into(),
                flags.into_iter().map(DescNode::from).collect(),
            ),
        };
        nested.push((*value.inner).into());
        DescNode { desc: header, nested_items: nested }
    }
}

impl From<ClassNode> for DescNode {
    fn from(value: ClassNode) -> Self {
        let negated = value.negated;
        let neg = if negated { "not a" } else { "a" };
        match value.kind {
            ClassKind::Dot       => DescNode::leaf("matches any character (except newline)"),
            ClassKind::PerlDigit => DescNode::leaf(format!("matches any character this is {} digit 0 to 9 in any unicode script", neg)),
            ClassKind::PerlSpace => DescNode::leaf(format!("matches any character that is {} whitespace character in any unicode script", neg)),
            ClassKind::PerlWord  => DescNode::leaf(format!("matches anything that is {} word character in any unicode script", neg)),
            ClassKind::AsciiAlnum  => DescNode::leaf(format!("matches any character that is {} alphanumeric ASCII, same as [0-9A-Za-z]", neg)),
            ClassKind::AsciiAlpha  => DescNode::leaf(format!("matches any character that is {} ASCII letter, same as [A-Za-z]", neg)),
            ClassKind::AsciiAscii  => DescNode::leaf(format!("matches any character that is {} ASCII character, same as [\\x00-\\x7F]", neg)),
            ClassKind::AsciiBlank  => DescNode::leaf(format!("matches any character that is {} space or tab", neg)),
            ClassKind::AsciiCntrl  => DescNode::leaf(format!("matches any character that is {} ASCII control character, same as [\\x00-\\x1F\\x7F]", neg)),
            ClassKind::AsciiDigit  => DescNode::leaf(format!("matches any character that is {} ASCII digit, same as [0-9]", neg)),
            ClassKind::AsciiGraph  => DescNode::leaf(format!("matches any character that is {} visible ASCII character, same as [!-~]", neg)),
            ClassKind::AsciiLower  => DescNode::leaf(format!("matches any character that is {} ASCII lowercase letter, same as [a-z]", neg)),
            ClassKind::AsciiPrint  => DescNode::leaf(format!("matches any character that is {} printable ASCII character, same as [ -~]", neg)),
            ClassKind::AsciiPunct  => DescNode::leaf(format!("matches any character that is {} ASCII punctuation, same as [!-/:-@\\[-`{{-~]", neg)),
            ClassKind::AsciiSpace  => DescNode::leaf(format!("matches any character that is {} ASCII whitespace, same as [ \\t\\r\\n\\f\\v]", neg)),
            ClassKind::AsciiUpper  => DescNode::leaf(format!("matches any character that is {} ASCII uppercase letter, same as [A-Z]", neg)),
            ClassKind::AsciiWord   => DescNode::leaf(format!("matches any character that is {} ASCII word character, same as [0-9A-Za-z_]", neg)),
            ClassKind::AsciiXdigit => DescNode::leaf(format!("matches any character that is {} hex digit, same as [0-9A-Fa-f]", neg)),
            ClassKind::Unicode(u) => DescNode::leaf(match u {
                UnicodeClassKind::Named(s) => {
                    let neg = if negated { "doesn't satisfy" } else { "satisfies" };
                    // i should explain the name as well, like Lu should become Uppercase_Letter, etc
                    // for single letter name too, N should become Number, etc
                    format!("matches any character that {} Unicode property \\p{{{}}}", neg, s)
                }
                UnicodeClassKind::NamedValue { negated: nv, name, value } => {
                    // i should explain the name as well, like sc sould become Script, gc should become General_Category, etc. but for now we just print the raw name and value.
                    format!("matches any character with {} {} {}", name, if nv^negated { "not equals to" } else { "equals to" }, value)
                }
            }),
            ClassKind::Bracketed(items) => DescNode {
                desc: format!("matches any one character that is {} in the list below", neg),
                nested_items: items.into_iter().map(DescNode::from).collect(),
            },
            ClassKind::BracketedOp { op, lhs, rhs } => {
                let neg = if negated { "not in" } else { "in" };
                let op_str = match op {
                    ClassBinaryOp::Intersection        => "both of the list below",
                    ClassBinaryOp::Difference          => "first list but not in second list",
                    ClassBinaryOp::SymmetricDifference => "first list or in second list but not in both lists",
                };
                let to_desc = |k: Box<ClassKind>| DescNode::from(ClassNode {
                    // need to find a representation that also preserves the start/end of lhs and rhs
                    // i think lsh and rsh should be vector of class items
                    span: Span { start: 0, end: 0 },
                    negated: false,
                    kind: *k,
                });
                DescNode {
                    desc: format!("matches any character that is {} {}", neg, op_str),
                    nested_items: vec![
                        DescNode{ desc: "1st list".into(), nested_items: vec![to_desc(lhs)] },
                        DescNode{ desc: "2nd list".into(), nested_items: vec![to_desc(rhs)] }
                    ],
                }
            }
        }
    }
}

impl From<ClassItem> for DescNode {
    fn from(item: ClassItem) -> Self {
        match item {
            ClassItem::Literal(l)             => l.into(),
            ClassItem::Range { start, end, .. } => DescNode::leaf(format!("matches anything from '{}' to '{}'", start, end)),
            ClassItem::Class(c)               => c.into(),
        }
    }
}

impl From<RepeatNode> for DescNode {
    fn from(value: RepeatNode) -> Self {
        let count = match (value.min, value.max) {
            (0, Some(1)) => "optionally".into(),
            (0, None)    => "0 or more times".into(),
            (1, None)    => "1 or more times".into(),
            (n, Some(m)) if n == m => format!("exactly {} time(s)", n),
            (lo, None)   => format!("{} or more times", lo),
            (lo, Some(hi)) => format!("{} to {} times", lo, hi),
        };
        let greedy = if value.greedy { " (greedy)" } else { " (lazy)" };
        let mut inner = DescNode::from(*value.inner);
        inner.desc.push_str(&format!(", {}{}", count, greedy));
        inner
    }
}
