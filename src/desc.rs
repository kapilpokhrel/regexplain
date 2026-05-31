use crate::types::*;

pub struct DescNode {
    pub desc: String,
    pub nested_items: Vec<DescNode>,
    pub match_str: String,
}

pub struct SymbolTable {
    pattern: String,
} // I don't know what else to call it lol

pub struct DescGenerator {
    sym_table: SymbolTable
}

impl DescGenerator {
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            sym_table: SymbolTable{pattern: pattern.into()}
        }
    }

    fn get_match_str_from_span(&self, s: Span) -> String {
        self.sym_table.pattern[s.start..s.end].into()
    }

    fn leaf_from_span(&self, s: Span, desc: impl Into<String>) -> DescNode {
        DescNode { match_str: self.get_match_str_from_span(s), desc: desc.into(), nested_items: vec![] }
    }

    fn leaf_from_str(&self, s: impl Into<String>, desc: impl Into<String>) -> DescNode {
        DescNode { match_str: s.into(), desc: desc.into(), nested_items: vec![] }
    }
}

pub trait Describer<T> {
    fn describe(&self, target: T) -> DescNode;
}

impl Describer<RegExplainSimplifiedNode> for DescGenerator {
    fn describe(&self, target: RegExplainSimplifiedNode) -> DescNode {
        match target {
            RegExplainSimplifiedNode::Flags(f)     => self.describe(f),
            RegExplainSimplifiedNode::Literal(l)   => self.describe(l),
            RegExplainSimplifiedNode::Assertion(a) => self.describe(a),
            RegExplainSimplifiedNode::Class(c)     => self.describe(c),
            RegExplainSimplifiedNode::Group(g)     => self.describe(g),
            RegExplainSimplifiedNode::Repeat(r)    => self.describe(r),
            RegExplainSimplifiedNode::Alt { alts, span } => DescNode {
                match_str: self.get_match_str_from_span(span),
                desc: "Selects one of the matches from the following list".into(),
                nested_items: alts.into_iter().map(|x| self.describe(x)).collect(),
            },
            RegExplainSimplifiedNode::Concat { span, nodes } => DescNode {
                match_str: self.get_match_str_from_span(span),
                desc: String::new(),
                nested_items: nodes.into_iter().map(|x| self.describe(x)).collect(),
            },
        }
    }
}

impl Describer<LiteralNode> for DescGenerator {
    fn describe(&self, target: LiteralNode) -> DescNode {
        self.leaf_from_span(target.span, match target.ch {
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

impl Describer<AssertionNode> for DescGenerator {
    fn describe(&self, target: AssertionNode) -> DescNode {
        self.leaf_from_span(target.span, match target.kind {
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

impl Describer<FlagItem> for DescGenerator {
    fn describe(&self, target: FlagItem) -> DescNode {
        let flag_desc = match target.kind {
            FlagKind::CaseInsensitive   => ('i', "case-insensitive matching"),
            FlagKind::MultiLine         => ('m', "multi-line (^ and $ match line boundaries)"),
            FlagKind::DotMatchesNewLine => ('s', "dot matches newline"),
            FlagKind::SwapGreed         => ('U', "swap greediness"),
            FlagKind::Unicode           => ('u', "Unicode mode"),
            FlagKind::Crlf              => ('R', "CRLF line endings"),
            FlagKind::IgnoreWhitespace  => ('x', "ignore whitespace and comments"),
        };
        self.leaf_from_str(flag_desc.0, format!(
            "{} {}",
            if target.negated { "disable" } else { "enable" },
            flag_desc.1
        ))
    }
}

impl Describer<FlagNode> for DescGenerator {
    fn describe(&self, target: FlagNode) -> DescNode {
        DescNode {
            match_str: self.get_match_str_from_span(target.span),
            desc: "enable/disable the following flags:".into(),
            nested_items: target.items.into_iter().map(|x| self.describe(x)).collect(),
        }
    }
}

impl Describer<GroupNode> for DescGenerator {
    fn describe(&self, target: GroupNode) -> DescNode {
        let (header, mut nested) = match target.kind {
            GroupKind::Capture { index, name: None } => {
                (format!("capture group #{}", index), vec![])
            }
            GroupKind::Capture { index, name: Some(name) } => {
                (format!("capture group #{} named \"{}\"", index, name), vec![])
            }
            GroupKind::NonCapturing(flags) => (
                "non-capturing group".into(),
                flags.into_iter().map(|x| self.describe(x)).collect(),
            ),
        };
        nested.push(self.describe(*target.inner));
        DescNode { match_str: self.get_match_str_from_span(target.span), desc: header, nested_items: nested }
    }
}

impl Describer<ClassNode> for DescGenerator {
    fn describe(&self, target: ClassNode) -> DescNode {
        let negated = target.negated;
        let neg = if negated { "not a" } else { "a" };
        match target.kind {
            ClassKind::Dot       => self.leaf_from_span(target.span, "matches any character (except newline)"),
            ClassKind::PerlDigit => self.leaf_from_span(target.span, format!("matches any character this is {} digit 0 to 9 in any unicode script", neg)),
            ClassKind::PerlSpace => self.leaf_from_span(target.span, format!("matches any character that is {} whitespace character in any unicode script", neg)),
            ClassKind::PerlWord  => self.leaf_from_span(target.span, format!("matches anything that is {} word character in any unicode script", neg)),
            ClassKind::AsciiAlnum  => self.leaf_from_span(target.span, format!("matches any character that is {} alphanumeric ASCII, same as [0-9A-Za-z]", neg)),
            ClassKind::AsciiAlpha  => self.leaf_from_span(target.span, format!("matches any character that is {} ASCII letter, same as [A-Za-z]", neg)),
            ClassKind::AsciiAscii  => self.leaf_from_span(target.span, format!("matches any character that is {} ASCII character, same as [\\x00-\\x7F]", neg)),
            ClassKind::AsciiBlank  => self.leaf_from_span(target.span, format!("matches any character that is {} space or tab", neg)),
            ClassKind::AsciiCntrl  => self.leaf_from_span(target.span, format!("matches any character that is {} ASCII control character, same as [\\x00-\\x1F\\x7F]", neg)),
            ClassKind::AsciiDigit  => self.leaf_from_span(target.span, format!("matches any character that is {} ASCII digit, same as [0-9]", neg)),
            ClassKind::AsciiGraph  => self.leaf_from_span(target.span, format!("matches any character that is {} visible ASCII character, same as [!-~]", neg)),
            ClassKind::AsciiLower  => self.leaf_from_span(target.span, format!("matches any character that is {} ASCII lowercase letter, same as [a-z]", neg)),
            ClassKind::AsciiPrint  => self.leaf_from_span(target.span, format!("matches any character that is {} printable ASCII character, same as [ -~]", neg)),
            ClassKind::AsciiPunct  => self.leaf_from_span(target.span, format!("matches any character that is {} ASCII punctuation, same as [!-/:-@\\[-`{{-~]", neg)),
            ClassKind::AsciiSpace  => self.leaf_from_span(target.span, format!("matches any character that is {} ASCII whitespace, same as [ \\t\\r\\n\\f\\v]", neg)),
            ClassKind::AsciiUpper  => self.leaf_from_span(target.span, format!("matches any character that is {} ASCII uppercase letter, same as [A-Z]", neg)),
            ClassKind::AsciiWord   => self.leaf_from_span(target.span, format!("matches any character that is {} ASCII word character, same as [0-9A-Za-z_]", neg)),
            ClassKind::AsciiXdigit => self.leaf_from_span(target.span, format!("matches any character that is {} hex digit, same as [0-9A-Fa-f]", neg)),
            ClassKind::Unicode(u) => self.leaf_from_span(target.span, match u {
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
            ClassKind::Bracketed(items) => {
                let neg = if negated { "not in" } else { "in" };
                DescNode {
                    match_str: self.get_match_str_from_span(target.span),
                    desc: format!("matches any one character that is {} the list below", neg),
                    nested_items: items.into_iter().map(|x| self.describe(x)).collect(),
                }
            },
            ClassKind::BracketedOp { op, lhs, rhs } => {
                let neg = if negated { "not in" } else { "in" };
                let op_str = match op {
                    ClassBinaryOp::Intersection        => "both of the list below",
                    ClassBinaryOp::Difference          => "first list but not in second list",
                    ClassBinaryOp::SymmetricDifference => "first list or in second list but not in both lists",
                };

                DescNode {
                    match_str: self.get_match_str_from_span(target.span),
                    desc: format!("matches any character that is {} {}", neg, op_str),
                    nested_items: vec![
                        DescNode {
                            match_str: self.get_match_str_from_span(lhs.span),
                            desc: "1st list".into(), nested_items: vec![self.describe(lhs)]
                        },
                        DescNode {
                            match_str: self.get_match_str_from_span(rhs.span),
                            desc: "2nd list".into(), nested_items: vec![self.describe(rhs)]
                        },
                    ],
                }
            }
        }

    }
}


impl Describer<ClassOperand> for DescGenerator {
    fn describe(&self, target: ClassOperand) -> DescNode {
        match *target.kind {
            ClassKind::Bracketed(items) => DescNode {
                match_str: self.get_match_str_from_span(target.span),
                desc: String::new(),
                nested_items: items.into_iter().map(|x| self.describe(x)).collect(),
            },
            other => self.describe(ClassNode { span: target.span, negated: false, kind: other }),
        }
    }
}

impl Describer<ClassItem> for DescGenerator {
    fn describe(&self, target: ClassItem) -> DescNode {
        match target {
            ClassItem::Literal(l)             => self.describe(l),
            ClassItem::Range { span, start, end } => self.leaf_from_span(span, format!("matches anything from '{}' to '{}'", start, end)),
            ClassItem::Class(c)               => self.describe(c),
        }
    }
}

impl Describer<RepeatNode> for DescGenerator {
    fn describe(&self, target: RepeatNode) -> DescNode {
        let greedy = if target.greedy { " (greedy)" } else { " (lazy)" };
        let count_eval = match (target.min, target.max) {
            (0, Some(1)) => "optionally".to_string() + greedy,
            (0, None)    => "0 or more times".to_string() + greedy,
            (1, None)    => "1 or more times".to_string() + greedy,
            (n, Some(m)) if n == m => format!("exactly {} time(s)", n), // no need to show lazy or greedy for exact
            (lo, None)   => format!("{} or more times", lo) + greedy,
            (lo, Some(hi)) => format!("{} to {} times", lo, hi) + greedy,
        };
        let mut inner = self.describe(*target.inner);
        inner.desc.push_str(&format!(", {}", count_eval));
        inner
    }
}

impl DescNode {
    pub fn print(&self, indent: usize) {
        if !self.desc.is_empty() {
            println!("{}`{}` {}", "  ".repeat(indent), self.match_str, self.desc);
        }
        let child_indent = if self.desc.is_empty() { indent } else { indent + 1 };
        for child in &self.nested_items {
            child.print(child_indent);
        }
    }
}
