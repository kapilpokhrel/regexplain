#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct RegExplainForm {
    pub pattern: String,
    pub root: RegExplainSimplifiedNode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RegExplainSimplifiedNode {
    Flags(FlagNode),
    Literal(LiteralNode),
    Assertion(AssertionNode),
    Alt   { span: PatternSpan, alts:  Vec<RegExplainSimplifiedNode> },
    Concat { span: PatternSpan, nodes: Vec<RegExplainSimplifiedNode> },
    Class(ClassNode),
    Group(GroupNode),
    Repeat(RepeatNode),
}

impl RegExplainSimplifiedNode {
    pub fn span(&self) -> PatternSpan {
        match self {
            Self::Flags(f)            => f.span,
            Self::Literal(l)          => l.span,
            Self::Assertion(a)        => a.span,
            Self::Class(c)            => c.span,
            Self::Group(g)            => g.span,
            Self::Repeat(r)           => r.span,
            Self::Alt { span, .. }    => *span,
            Self::Concat { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepeatNode {
    pub span: PatternSpan,
    pub greedy: bool,
    pub min: u32,
    /// none means unbounded.
    pub max: Option<u32>,
    pub inner: Box<RegExplainSimplifiedNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupNode {
    pub span: PatternSpan,
    pub kind: GroupKind,
    pub inner: Box<RegExplainSimplifiedNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GroupKind {
    Capture { index: u32, name: Option<String> },
    NonCapturing(Vec<FlagItem>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassNode {
    pub span: PatternSpan,
    pub negated: bool,
    pub kind: ClassKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClassKind {
    Dot,
    PerlDigit,
    PerlSpace,
    PerlWord,
    AsciiAlnum,
    AsciiAlpha,
    AsciiAscii,
    AsciiBlank,
    AsciiCntrl,
    AsciiDigit,
    AsciiGraph,
    AsciiLower,
    AsciiPrint,
    AsciiPunct,
    AsciiSpace,
    AsciiUpper,
    AsciiWord,
    AsciiXdigit,
    Unicode(UnicodeClassKind),
    Bracketed(Vec<ClassItem>),
    BracketedOp { op: ClassBinaryOp, lhs: ClassOperand, rhs: ClassOperand },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassOperand {
    pub span: PatternSpan,
    pub kind: Box<ClassKind>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClassItem {
    Literal(LiteralNode),
    Range { span: PatternSpan, start: char, end: char },
    Class(ClassNode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassBinaryOp {
    Intersection,        // [a&&b]
    Difference,          // [a--b]
    SymmetricDifference, // [a~~b]
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnicodeClassKind {
    Named(String),
    NamedValue {
        negated: bool,
        name: String,
        value: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiteralNode {
    pub span: PatternSpan,
    pub ch: LiteralChar,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LiteralChar {
    Verbatim(String),
    Octal(String),
    Hex(String),
    Special(SpecialChar),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialChar {
    Bell,
    FormFeed,
    Tab,
    LineFeed,
    CarriageReturn,
    VerticalTab,
    Space,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssertionNode {
    pub span: PatternSpan,
    pub kind: AssertionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssertionKind {
    StartLine,
    EndLine,
    StartText,
    EndText,
    WordBoundary,
    NotWordBoundary,
    WordBoundaryStart,
    WordBoundaryEnd,
    WordBoundaryStartHalf,
    WordBoundaryEndHalf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlagNode {
    pub span: PatternSpan,
    pub items: Vec<FlagItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlagItem {
    pub span: PatternSpan,
    pub negated: bool,
    pub kind: FlagKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagKind {
    CaseInsensitive,   // i
    MultiLine,         // m
    DotMatchesNewLine, // s
    SwapGreed,         // U
    Unicode,           // u
    Crlf,              // R
    IgnoreWhitespace,  // x
}
