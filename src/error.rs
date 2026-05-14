use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    UnexpectedEof,
    UnexpectedToken { expected: &'static str, found: String },
    InvalidNumber(String),
    InvalidString(String),
    Message(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub span: Span,
    pub kind: ParseErrorKind,
}

impl ParseError {
    pub fn new(span: Span, kind: ParseErrorKind) -> Self {
        Self { span, kind }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ParseErrorKind::UnexpectedEof => {
                write!(f, "unexpected end of file at {}..{}", self.span.start, self.span.end)
            }
            ParseErrorKind::UnexpectedToken { expected, found } => write!(
                f,
                "unexpected token `{found}` at {}..{}, expected {expected}",
                self.span.start, self.span.end
            ),
            ParseErrorKind::InvalidNumber(v) => {
                write!(f, "invalid number `{v}` at {}..{}", self.span.start, self.span.end)
            }
            ParseErrorKind::InvalidString(v) => {
                write!(f, "invalid string `{v}` at {}..{}", self.span.start, self.span.end)
            }
            ParseErrorKind::Message(msg) => write!(f, "{msg} at {}..{}", self.span.start, self.span.end),
        }
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeErrorKind {
    Parse(ParseErrorKind),
    DuplicateSymbol(String),
    UnknownSymbol(String),
    TypeMismatch { expected: String, found: String },
    Validation(String),
    InsufficientBalance { name: String, requested: u128, available: u128 },
    CertificationFailed { name: String, ratio: String, threshold: String },
    ProjectionMissing(String),
    ProjectionConsumed(String),
    ContractMissing(String),
    RecordMissing(String),
    InvalidOperation(String),
    ImmutableViolation(String),
    ZeroNormalization,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    pub span: Option<Span>,
    pub kind: RuntimeErrorKind,
}

impl RuntimeError {
    pub fn new(kind: RuntimeErrorKind) -> Self {
        Self { span: None, kind }
    }

    pub fn with_span(kind: RuntimeErrorKind, span: Span) -> Self {
        Self { span: Some(span), kind }
    }
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            RuntimeErrorKind::Parse(k) => write!(f, "parse error: {:?}", k),
            RuntimeErrorKind::DuplicateSymbol(s) => write!(f, "duplicate symbol `{s}`"),
            RuntimeErrorKind::UnknownSymbol(s) => write!(f, "unknown symbol `{s}`"),
            RuntimeErrorKind::TypeMismatch { expected, found } => {
                write!(f, "type mismatch: expected {expected}, found {found}")
            }
            RuntimeErrorKind::Validation(msg) => write!(f, "validation failed: {msg}"),
            RuntimeErrorKind::InsufficientBalance { name, requested, available } => write!(
                f,
                "insufficient balance in `{name}`: requested {requested}, available {available}"
            ),
            RuntimeErrorKind::CertificationFailed { name, ratio, threshold } => write!(
                f,
                "certification failed for `{name}`: ratio {ratio} below threshold {threshold}"
            ),
            RuntimeErrorKind::ProjectionMissing(id) => write!(f, "projection `{id}` not found"),
            RuntimeErrorKind::ProjectionConsumed(id) => write!(f, "projection `{id}` was already consumed"),
            RuntimeErrorKind::ContractMissing(id) => write!(f, "contract `{id}` not found"),
            RuntimeErrorKind::RecordMissing(id) => write!(f, "record `{id}` not found"),
            RuntimeErrorKind::InvalidOperation(msg) => write!(f, "invalid operation: {msg}"),
            RuntimeErrorKind::ImmutableViolation(msg) => write!(f, "immutable violation: {msg}"),
            RuntimeErrorKind::ZeroNormalization => write!(f, "normalization of the zero vector is undefined"),
        }
    }
}

impl std::error::Error for RuntimeError {}
