use crate::error::{ParseError, ParseErrorKind, Span};
use crate::token::{Token, TokenKind};

const KEYWORDS: &[(&str, TokenKind)] = &[
    ("vector", TokenKind::Vector),
    ("wallet", TokenKind::Wallet),
    ("bind", TokenKind::Bind),
    ("certify", TokenKind::Certify),
    ("transfer", TokenKind::Transfer),
    ("drain", TokenKind::Drain),
    ("project", TokenKind::Project),
    ("reconstruct", TokenKind::Reconstruct),
    ("query", TokenKind::Query),
    ("record", TokenKind::Record),
    ("contract", TokenKind::Contract),
    ("action", TokenKind::Action),
    ("with", TokenKind::With),
    ("to", TokenKind::To),
    ("amount", TokenKind::Amount),
    ("by", TokenKind::By),
    ("into", TokenKind::Into),
    ("from", TokenKind::From),
    ("policy", TokenKind::Policy),
    ("ctx", TokenKind::Ctx),
    ("space", TokenKind::Space),
    ("op", TokenKind::Op),
    ("risk", TokenKind::Risk),
    ("threshold_version", TokenKind::ThresholdVersion),
    ("outcome", TokenKind::Outcome),
    ("gain", TokenKind::Gain),
    ("loss", TokenKind::Loss),
    ("partial", TokenKind::Partial),
    ("release", TokenKind::Release),
    ("staged", TokenKind::Staged),
    ("as", TokenKind::As),
];

#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self { src, bytes: src.as_bytes(), pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<u8> {
        self.bytes.get(self.pos + 1).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let ch = self.peek()?;
        self.pos += 1;
        Some(ch)
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
                self.pos += 1;
            }

            let mut skipped = false;

            if self.peek() == Some(b'/') && self.peek_next() == Some(b'/') {
                skipped = true;
                self.pos += 2;
                while let Some(ch) = self.peek() {
                    self.pos += 1;
                    if ch == b'\n' {
                        break;
                    }
                }
            } else if self.peek() == Some(b'#') {
                skipped = true;
                self.pos += 1;
                while let Some(ch) = self.peek() {
                    self.pos += 1;
                    if ch == b'\n' {
                        break;
                    }
                }
            }

            if !skipped {
                break;
            }
        }
    }

    fn lex_number(&mut self, start: usize) -> Result<Token, ParseError> {
        let mut end = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.pos += 1;
            end = self.pos;
        }
        let text = &self.src[start..end];
        let value = text.parse::<u128>().map_err(|_| {
            ParseError::new(Span::new(start, end), ParseErrorKind::InvalidNumber(text.to_string()))
        })?;
        Ok(Token::new(TokenKind::Integer(value), start, end))
    }

    fn lex_ident_or_keyword(&mut self, start: usize) -> Token {
        let mut end = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_') {
            self.pos += 1;
            end = self.pos;
        }
        let text = &self.src[start..end];
        if let Some((_, keyword)) = KEYWORDS.iter().find(|(k, _)| *k == text) {
            Token::new(keyword.clone(), start, end)
        } else {
            Token::new(TokenKind::Ident(text.to_string()), start, end)
        }
    }

    fn lex_string(&mut self, start: usize) -> Result<Token, ParseError> {
        let mut out = String::new();
        loop {
            let ch = self.bump().ok_or_else(|| {
                ParseError::new(Span::new(start, self.pos), ParseErrorKind::UnexpectedEof)
            })?;
            match ch {
                b'"' => break,
                b'\\' => {
                    let esc = self.bump().ok_or_else(|| {
                        ParseError::new(Span::new(start, self.pos), ParseErrorKind::UnexpectedEof)
                    })?;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        other => out.push(other as char),
                    }
                }
                other => out.push(other as char),
            }
        }
        Ok(Token::new(TokenKind::String(out), start, self.pos))
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.skip_ws_and_comments();
        let start = self.pos;
        let ch = self.bump()?;

        let token = match ch {
            b'(' => Ok(Token::new(TokenKind::LParen, start, self.pos)),
            b')' => Ok(Token::new(TokenKind::RParen, start, self.pos)),
            b'{' => Ok(Token::new(TokenKind::LBrace, start, self.pos)),
            b'}' => Ok(Token::new(TokenKind::RBrace, start, self.pos)),
            b':' => Ok(Token::new(TokenKind::Colon, start, self.pos)),
            b';' => Ok(Token::new(TokenKind::Semi, start, self.pos)),
            b',' => Ok(Token::new(TokenKind::Comma, start, self.pos)),
            b'.' => Ok(Token::new(TokenKind::Dot, start, self.pos)),
            b'=' => Ok(Token::new(TokenKind::Eq, start, self.pos)),
            b'"' => self.lex_string(start),
            b'0'..=b'9' => self.lex_number(start),
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => Ok(self.lex_ident_or_keyword(start)),
            other => Err(ParseError::new(
                Span::new(start, self.pos),
                ParseErrorKind::Message(format!("unexpected character `{}`", other as char)),
            )),
        };

        Some(token)
    }
}

pub fn lex(src: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    for item in Lexer::new(src) {
        tokens.push(item?);
    }
    let len = src.len();
    tokens.push(Token::new(TokenKind::Eof, len, len));
    Ok(tokens)
}
