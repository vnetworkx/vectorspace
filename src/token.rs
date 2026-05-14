#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Vector,
    Wallet,
    Bind,
    Certify,
    Transfer,
    Drain,
    Project,
    Reconstruct,
    Query,
    Record,
    Contract,
    Action,
    With,
    To,
    Amount,
    By,
    Into,
    From,
    Policy,
    Ctx,
    Space,
    Op,
    Risk,
    ThresholdVersion,
    Outcome,
    Gain,
    Loss,
    Partial,
    Release,
    Staged,
    As,
    Eq,
    Colon,
    Semi,
    Comma,
    Dot,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Integer(u128),
    String(String),
    Ident(String),
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: usize,
    pub end: usize,
}

impl Token {
    pub fn new(kind: TokenKind, start: usize, end: usize) -> Self {
        Self { kind, start, end }
    }
}
