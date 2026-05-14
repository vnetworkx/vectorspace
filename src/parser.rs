use crate::ast::*;
use crate::error::{ParseError, ParseErrorKind, Span};
use crate::lexer::lex;
use crate::token::{Token, TokenKind};

#[derive(Debug, Clone)]
pub struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, index: 0 }
    }

    pub fn from_source(source: &str) -> Result<Self, ParseError> {
        Ok(Self::new(lex(source)?))
    }

    fn current(&self) -> &Token {
        &self.tokens[self.index]
    }

    fn at_end(&self) -> bool {
        self.kind_eq(&TokenKind::Eof)
    }


    fn kind_eq(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.current().kind) == std::mem::discriminant(kind)
    }

    fn advance(&mut self) {
        if !self.at_end() {
            self.index += 1;
        }
    }

    fn error_here(&self, expected: &'static str) -> ParseError {
        let token = self.current();
        ParseError::new(
            Span::new(token.start, token.end),
            ParseErrorKind::UnexpectedToken {
                expected,
                found: format!("{:?}", token.kind),
            },
        )
    }

    fn expect_semi(&mut self) -> Result<(), ParseError> {
        if self.kind_eq(&TokenKind::Semi) {
            self.advance();
            Ok(())
        } else {
            Err(self.error_here("`;`"))
        }
    }

    fn expect_kind(&mut self, kind: fn(&TokenKind) -> bool, expected: &'static str) -> Result<(), ParseError> {
        if kind(&self.current().kind) {
            self.advance();
            Ok(())
        } else {
            Err(self.error_here(expected))
        }
    }

    fn take_ident(&mut self) -> Result<String, ParseError> {
        match &self.current().kind {
            TokenKind::Ident(text) => {
                let out = text.clone();
                self.advance();
                Ok(out)
            }
            other => Err(ParseError::new(
                Span::new(self.current().start, self.current().end),
                ParseErrorKind::UnexpectedToken {
                    expected: "identifier",
                    found: format!("{other:?}"),
                },
            )),
        }
    }

    fn take_string_like(&mut self) -> Result<String, ParseError> {
        match &self.current().kind {
            TokenKind::String(text) | TokenKind::Ident(text) => {
                let out = text.clone();
                self.advance();
                Ok(out)
            }
            TokenKind::Integer(v) => {
                let out = v.to_string();
                self.advance();
                Ok(out)
            }
            other => Err(ParseError::new(
                Span::new(self.current().start, self.current().end),
                ParseErrorKind::UnexpectedToken {
                    expected: "string or identifier",
                    found: format!("{other:?}"),
                },
            )),
        }
    }

    fn take_integer(&mut self) -> Result<u128, ParseError> {
        match &self.current().kind {
            TokenKind::Integer(v) => {
                let out = *v;
                self.advance();
                Ok(out)
            }
            other => Err(ParseError::new(
                Span::new(self.current().start, self.current().end),
                ParseErrorKind::UnexpectedToken {
                    expected: "integer",
                    found: format!("{other:?}"),
                },
            )),
        }
    }

    fn parse_vector_type(&mut self) -> Result<VectorType, ParseError> {
        let name = self.take_ident()?;
        VectorType::parse(&name).ok_or_else(|| {
            ParseError::new(
                Span::new(self.current().start, self.current().end),
                ParseErrorKind::Message(format!("unknown vector type `{name}`")),
            )
        })
    }

    fn parse_vector_literal(&mut self) -> Result<Vec<u128>, ParseError> {
        self.expect_kind(|k| k == &TokenKind::LParen, "`(`")?;
        let mut components = Vec::new();
        if !self.kind_eq(&TokenKind::RParen) {
            loop {
                let value = self.take_integer()?;
                components.push(value);
                if self.kind_eq(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect_kind(|k| k == &TokenKind::RParen, "`)`")?;
        Ok(components)
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_path_or_primary()
    }

    fn parse_path_or_primary(&mut self) -> Result<Expr, ParseError> {
        let mut expr = match &self.current().kind {
            TokenKind::Integer(v) => {
                let v = *v;
                self.advance();
                Expr::Integer(v)
            }
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Expr::String(s)
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                if self.kind_eq(&TokenKind::LParen) {
                    self.advance();
                    let args = self.parse_named_args(TokenKind::RParen)?;
                    self.expect_kind(|k| k == &TokenKind::RParen, "`)`")?;
                    Expr::Call { callee: name, args }
                } else {
                    Expr::Ident(name)
                }
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect_kind(|k| k == &TokenKind::RParen, "`)`")?;
                Expr::Group(Box::new(expr))
            }
            other => {
                return Err(ParseError::new(
                    Span::new(self.current().start, self.current().end),
                    ParseErrorKind::UnexpectedToken {
                        expected: "expression",
                        found: format!("{other:?}"),
                    },
                ))
            }
        };

        while self.kind_eq(&TokenKind::Dot) {
            self.advance();
            let part = self.take_ident()?;
            expr = match expr {
                Expr::Ident(first) => Expr::Path(vec![first, part]),
                Expr::Path(mut parts) => {
                    parts.push(part);
                    Expr::Path(parts)
                }
                other => {
                    return Err(ParseError::new(
                        Span::new(self.current().start, self.current().end),
                        ParseErrorKind::Message(format!("cannot apply path access to {other:?}")),
                    ))
                }
            };
        }

        Ok(expr)
    }

    fn parse_named_args(&mut self, terminator: TokenKind) -> Result<Vec<NamedArg>, ParseError> {
        let mut args = Vec::new();
        if self.kind_eq(&terminator) {
            return Ok(args);
        }
        loop {
            let arg = if let TokenKind::Ident(name) = &self.current().kind {
                let name = name.clone();
                let save = self.index;
                self.advance();
                if self.kind_eq(&TokenKind::Eq) {
                    self.advance();
                    let value = self.parse_expr()?;
                    NamedArg { name: Some(name), value }
                } else {
                    self.index = save;
                    let value = self.parse_expr()?;
                    NamedArg { name: None, value }
                }
            } else {
                let value = self.parse_expr()?;
                NamedArg { name: None, value }
            };
            args.push(arg);

            if self.kind_eq(&TokenKind::Comma) {
                self.advance();
                if self.kind_eq(&terminator) {
                    break;
                }
            } else {
                break;
            }
        }
        Ok(args)
    }

    fn parse_context_expr(&mut self) -> Result<ContextExpr, ParseError> {
        if !self.kind_eq(&TokenKind::Ctx) {
            return Err(self.error_here("ctx(...)"));
        }
        self.advance();
        self.expect_kind(|k| k == &TokenKind::LParen, "`(`")?;
        let args = self.parse_named_args(TokenKind::RParen)?;
        self.expect_kind(|k| k == &TokenKind::RParen, "`)`")?;
        Ok(ContextExpr::new(args))
    }

    fn parse_contract_action(&mut self) -> Result<ContractAction, ParseError> {
        self.expect_kind(|k| k == &TokenKind::Action, "`action`")?;
        let name = self.take_ident()?;
        let mut args = Vec::new();
        if self.kind_eq(&TokenKind::LParen) {
            self.advance();
            args = self.parse_named_args(TokenKind::RParen)?;
            self.expect_kind(|k| k == &TokenKind::RParen, "`)`")?;
        }
        self.expect_semi()?;
        Ok(ContractAction { name, args })
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match &self.current().kind {
            TokenKind::Vector => {
                self.advance();
                let name = self.take_ident()?;
                self.expect_kind(|k| k == &TokenKind::Colon, "`:`")?;
                let vector_type = self.parse_vector_type()?;
                self.expect_kind(|k| k == &TokenKind::Eq, "`=`")?;
                let components = self.parse_vector_literal()?;
                self.expect_semi()?;
                Ok(Statement::VectorDecl(VectorDecl { name, vector_type, components }))
            }
            TokenKind::Wallet => {
                self.advance();
                let name = self.take_ident()?;
                self.expect_kind(|k| k == &TokenKind::Eq, "`=`")?;
                if !self.kind_eq(&TokenKind::Bind) {
                    return Err(self.error_here("bind(...)"));
                }
                self.advance();
                self.expect_kind(|k| k == &TokenKind::LParen, "`(`")?;
                let public_key = self.take_string_like()?;
                self.expect_kind(|k| k == &TokenKind::RParen, "`)`")?;
                self.expect_semi()?;
                Ok(Statement::WalletDecl(WalletDecl { name, public_key }))
            }
            TokenKind::Certify => {
                self.advance();
                let target = self.take_ident()?;
                if !self.kind_eq(&TokenKind::With) {
                    return Err(self.error_here("with ctx(...)"));
                }
                self.advance();
                let context = self.parse_context_expr()?;
                self.expect_semi()?;
                Ok(Statement::Certify(CertifyStmt { target, context }))
            }
            TokenKind::Transfer => {
                self.advance();
                let source = self.take_ident()?;
                if !self.kind_eq(&TokenKind::To) {
                    return Err(self.error_here("to"));
                }
                self.advance();
                let destination = self.take_ident()?;
                if !self.kind_eq(&TokenKind::Amount) {
                    return Err(self.error_here("amount"));
                }
                self.advance();
                let amount = self.parse_expr()?;
                let mut drain = None;
                let mut policy = None;
                while !self.kind_eq(&TokenKind::Semi) {
                    match &self.current().kind {
                        TokenKind::Drain => {
                            self.advance();
                            drain = Some(self.parse_expr()?);
                        }
                        TokenKind::Policy => {
                            self.advance();
                            policy = Some(self.parse_expr()?);
                        }
                        other => {
                            return Err(ParseError::new(
                                Span::new(self.current().start, self.current().end),
                                ParseErrorKind::UnexpectedToken {
                                    expected: "`drain` or `policy` or `;`",
                                    found: format!("{other:?}"),
                                },
                            ))
                        }
                    }
                }
                self.expect_semi()?;
                Ok(Statement::Transfer(TransferStmt { source, destination, amount, drain, policy }))
            }
            TokenKind::Drain => {
                self.advance();
                let target = self.take_ident()?;
                if !self.kind_eq(&TokenKind::By) {
                    return Err(self.error_here("by"));
                }
                self.advance();
                let amount = self.parse_expr()?;
                self.expect_semi()?;
                Ok(Statement::Drain(DrainStmt { target, amount }))
            }
            TokenKind::Project => {
                self.advance();
                let source = self.take_ident()?;
                if !self.kind_eq(&TokenKind::Into) {
                    return Err(self.error_here("into"));
                }
                self.advance();
                let environment = self.take_ident()?;
                if !self.kind_eq(&TokenKind::Amount) {
                    return Err(self.error_here("amount"));
                }
                self.advance();
                let amount = self.parse_expr()?;
                let mut policy = None;
                while !self.kind_eq(&TokenKind::Semi) {
                    match &self.current().kind {
                        TokenKind::Policy => {
                            self.advance();
                            policy = Some(self.parse_expr()?);
                        }
                        other => {
                            return Err(ParseError::new(
                                Span::new(self.current().start, self.current().end),
                                ParseErrorKind::UnexpectedToken {
                                    expected: "`policy` or `;`",
                                    found: format!("{other:?}"),
                                },
                            ))
                        }
                    }
                }
                self.expect_semi()?;
                Ok(Statement::Project(ProjectStmt { source, environment, amount, policy }))
            }
            TokenKind::Reconstruct => {
                self.advance();
                let target = self.take_ident()?;
                if !self.kind_eq(&TokenKind::From) {
                    return Err(self.error_here("from"));
                }
                self.advance();
                let projection_id = self.take_ident()?;
                self.expect_semi()?;
                Ok(Statement::Reconstruct(ReconstructStmt { target, projection_id }))
            }
            TokenKind::Query => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect_semi()?;
                Ok(Statement::Query(QueryStmt { expr }))
            }
            TokenKind::Record => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect_semi()?;
                Ok(Statement::Record(RecordStmt { expr }))
            }
            TokenKind::Contract => {
                self.advance();
                let name = self.take_ident()?;
                self.expect_kind(|k| k == &TokenKind::LBrace, "`{`")?;
                let mut actions = Vec::new();
                while !self.kind_eq(&TokenKind::RBrace) {
                    actions.push(self.parse_contract_action()?);
                }
                self.expect_kind(|k| k == &TokenKind::RBrace, "`}`")?;
                Ok(Statement::Contract(ContractDecl { name, actions }))
            }
            other => Err(ParseError::new(
                Span::new(self.current().start, self.current().end),
                ParseErrorKind::UnexpectedToken {
                    expected: "statement",
                    found: format!("{other:?}"),
                },
            )),
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut statements = Vec::new();
        while !self.at_end() {
            statements.push(self.parse_statement()?);
        }
        Ok(Program { statements })
    }
}

pub fn parse(source: &str) -> Result<Program, ParseError> {
    let mut parser = Parser::from_source(source)?;
    parser.parse_program()
}
