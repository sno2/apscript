use codespan_reporting::diagnostic::{Diagnostic, Label};

use crate::{
    ast::{ElseIf, Expr, Node, Procedure, Span, Stmt, UnaryOpKind},
    lexer::{Keyword, Lexer, Token},
};

#[derive(Debug)]
pub struct Parser<'a, T: Copy> {
    pub lex: Lexer<'a>,
    pub fid: T,
    pub diagnostics: Vec<Diagnostic<T>>,
}

pub type Result<T> = std::result::Result<T, ()>;

impl<'a, T: Copy> Parser<'a, T> {
    pub fn new(fid: T, buffer: &'a [u8]) -> Self {
        Self {
            lex: Lexer::new(buffer),
            fid,
            diagnostics: Vec::new(),
        }
    }

    fn eat(&mut self, tok: Token) -> Result<Span> {
        if self.lex.token != tok {
            self.diagnostics.push(
                Diagnostic::error()
                    .with_message(format!(
                        "expected {}, found {}",
                        tok.as_ref(),
                        self.lex.token.as_ref()
                    ))
                    .with_labels(vec![Label::primary(self.fid, self.lex.span())
                        .with_message(format!("expected {}", tok.as_ref()))]),
            );
            return Err(());
        }
        let span = self.lex.span();
        self.lex.next();
        Ok(span)
    }

    fn parse_simple_expr(&mut self) -> Result<Expr> {
        Ok(match self.lex.token {
            Token::Keyword(Keyword::True) => {
                let start = self.lex.start as u32;
                self.lex.next();
                Expr::True {
                    start: start as u32,
                }
            }
            Token::Keyword(Keyword::False) => {
                let start = self.lex.start as u32;
                self.lex.next();
                Expr::False { start }
            }
            Token::Identifier => {
                let span = self.lex.span();
                self.lex.next();
                Expr::Identifier { span }
            }
            Token::StringLiteral => {
                let span = self.lex.span();
                self.lex.next();
                Expr::StringLiteral { span }
            }
            Token::LeftParen => {
                let start = self.lex.start as u32;
                self.lex.next();
                let inner = self.parse_expr(0)?;
                let end = self.lex.start as u32;
                self.eat(Token::RightParen)?;
                Expr::Paren {
                    span: Span { start, end },
                    value: Box::new(inner),
                }
            }
            Token::LeftBrack => {
                let start = self.lex.start as u32;
                self.lex.next();
                let mut values = Vec::new();

                loop {
                    if self.lex.token == Token::RightBrack {
                        break;
                    }

                    values.push(self.parse_expr(0)?);

                    if self.lex.token == Token::Comma {
                        self.lex.next();
                    }
                }

                let end = self.lex.index as u32;
                self.eat(Token::RightBrack)?;

                Expr::ArrayLiteral {
                    span: Span { start, end },
                    values: values.into_boxed_slice(),
                }
            }
            Token::Keyword(Keyword::Not) => {
                let start = self.lex.start as u32;
                self.lex.next();
                let e = self.parse_expr(70)?;
                Expr::UnaryOp {
                    span: Span {
                        start,
                        end: e.span().end,
                    },
                    kind: UnaryOpKind::Not,
                    value: Box::new(e),
                }
            }
            Token::Add => {
                let start = self.lex.start as u32;
                self.lex.next();
                let e = self.parse_expr(70)?;
                Expr::UnaryOp {
                    span: Span {
                        start,
                        end: e.span().end,
                    },
                    kind: UnaryOpKind::Pos,
                    value: Box::new(e),
                }
            }
            Token::Sub => {
                let start = self.lex.start as u32;
                self.lex.next();
                let e = self.parse_expr(70)?;
                Expr::UnaryOp {
                    span: Span {
                        start,
                        end: e.span().end,
                    },
                    kind: UnaryOpKind::Neg,
                    value: Box::new(e),
                }
            }
            Token::IntegerLiteral => {
                let span = self.lex.span();
                self.lex.next();
                Expr::IntegerLiteral { span }
            }
            Token::FloatLiteral => {
                let span = self.lex.span();
                self.lex.next();
                Expr::FloatLiteral { span }
            }
            Token::HexLiteral => {
                let span = self.lex.span();
                self.lex.next();
                Expr::HexLiteral { span }
            }
            Token::BinaryLiteral => {
                let span = self.lex.span();
                self.lex.next();
                Expr::BinaryLiteral { span }
            }
            tok => {
                self.diagnostics.push(
                    Diagnostic::error()
                        .with_message(format!("expected expression, found {}", tok.as_ref()))
                        .with_labels(vec![Label::primary(self.fid, self.lex.span())
                            .with_message("expected expression")]),
                );
                return Err(());
            }
        })
    }

    pub fn parse_expr(&mut self, lbp: u8) -> Result<Expr> {
        let mut lhs = self.parse_simple_expr()?;

        loop {
            let prec = self.lex.token.lbp();

            if prec == 0 || prec < lbp {
                break;
            }

            if self.lex.token == Token::LeftBrack {
                self.lex.next();
                let index = self.parse_expr(0)?;
                let end = self.lex.index as u32;
                self.eat(Token::RightBrack)?;
                lhs = Expr::Index {
                    span: Span {
                        start: lhs.span().start,
                        end,
                    },
                    value: Box::new(lhs),
                    index: Box::new(index),
                };
                continue;
            }

            if self.lex.token == Token::LeftParen {
                self.lex.next();

                let mut args = Vec::new();

                loop {
                    if self.lex.token == Token::RightParen {
                        break;
                    }

                    args.push(self.parse_expr(0)?);

                    if self.lex.token == Token::Comma {
                        self.lex.next();
                    }
                }

                let end = self.lex.index as u32;
                self.eat(Token::RightParen)?;

                lhs = Expr::FnCall {
                    span: Span {
                        start: lhs.span().start,
                        end,
                    },
                    calle: Box::new(lhs),
                    args: args.into_boxed_slice(),
                };
                continue;
            }

            let kind = self.lex.token.into();
            self.lex.next();

            lhs = Expr::BinaryOp {
                kind,
                lhs: Box::new(lhs),
                rhs: Box::new(self.parse_expr(prec)?),
            };
        }

        Ok(lhs)
    }

    fn expect_stmt_end(&mut self, node: &impl Node) {
        if !self.lex.has_newline_before && self.lex.token != Token::EOF {
            self.diagnostics.push(
                Diagnostic::error()
                    .with_message(format!(
                        "expected new line after statement, found {}",
                        self.lex.token.as_ref()
                    ))
                    .with_labels(vec![
                        Label::primary(self.fid, self.lex.span())
                            .with_message("expected new line here"),
                        Label::secondary(self.fid, node.span()).with_message("main statement here"),
                    ]),
            );
        }
    }

    pub fn parse_scope(&mut self, is_global_scope: bool) -> Result<Box<[Stmt]>> {
        let mut nodes = Vec::new();

        loop {
            match self.lex.token {
                Token::Identifier => {
                    let name = self.lex.span();
                    self.lex.next();

                    match self.lex.token {
                        Token::ThinArrow => {
                            self.lex.next();
                            let value = self.parse_expr(0)?;
                            let stmt = Stmt::VarAssign { name, value };
                            self.expect_stmt_end(&stmt);
                            nodes.push(stmt);
                        }
                        _ => 'blk: {
                            self.lex.index = name.start as usize;
                            self.lex.next();
                            let value = self.parse_expr(0)?;

                            if self.lex.token == Token::ThinArrow {
                                self.lex.next();
                                let Expr::Index { value: root, index, .. } = value else {
									self.expect_stmt_end(&value);
									return Err(());
								};

                                let value = self.parse_expr(0)?;
                                let node = Stmt::IndexAssign { root, index, value };
                                self.expect_stmt_end(&node);
                                nodes.push(node);
                                break 'blk;
                            }
                            self.expect_stmt_end(&value);
                            nodes.push(Stmt::Expr(value));
                        }
                    }
                }
                Token::Add
                | Token::Sub
                | Token::IntegerLiteral
                | Token::LeftBrack
                | Token::LeftParen => 'blk: {
                    let value = self.parse_expr(0)?;

                    if self.lex.token == Token::ThinArrow {
                        println!("HERE!");
                        self.lex.next();
                        let Expr::Index { value: root, index, .. } = value else {
							self.expect_stmt_end(&value);
							nodes.push(Stmt::Expr(value));
							break 'blk;
						};

                        let value = self.parse_expr(0)?;
                        let node = Stmt::IndexAssign { root, index, value };
                        self.expect_stmt_end(&node);
                        nodes.push(node);
                        break 'blk;
                    }

                    self.expect_stmt_end(&value);
                    nodes.push(Stmt::Expr(value));
                }
                Token::Keyword(Keyword::If) => {
                    self.lex.next();

                    self.eat(Token::LeftParen)?;
                    let cond = self.parse_expr(0)?;
                    self.eat(Token::RightParen)?;
                    self.eat(Token::LeftBrace)?;
                    let scope = self.parse_scope(is_global_scope)?;
                    self.eat(Token::RightBrace)?;

                    let mut else_ifs = Vec::new();
                    let mut els = None;

                    loop {
                        if self.lex.token != Token::Keyword(Keyword::Else) {
                            break;
                        }
                        self.lex.next();

                        if self.lex.token == Token::LeftBrace {
                            self.lex.next();
                            els = Some(self.parse_scope(is_global_scope)?);
                            self.eat(Token::RightBrace)?;
                            break;
                        }

                        self.eat(Token::Keyword(Keyword::If))?;
                        self.eat(Token::LeftParen)?;
                        let cond = self.parse_expr(0)?;
                        self.eat(Token::RightParen)?;
                        self.eat(Token::LeftBrace)?;
                        let scope = self.parse_scope(is_global_scope)?;
                        self.eat(Token::RightBrace)?;
                        else_ifs.push(ElseIf { cond, scope });
                    }

                    nodes.push(Stmt::If {
                        cond: Box::new(cond),
                        scope,
                        else_ifs: else_ifs.into_boxed_slice(),
                        els,
                    });
                }
                Token::Keyword(Keyword::Procedure) => {
                    let start = self.lex.start as u32;
                    self.lex.next();
                    let name = self.eat(Token::Identifier)?;
                    self.eat(Token::LeftParen)?;
                    self.eat(Token::RightParen)?;
                    self.eat(Token::LeftBrace)?;
                    let scope = self.parse_scope(false)?;
                    let end = self.lex.index as u32;
                    self.eat(Token::RightBrace)?;

                    if !is_global_scope {
                        self.diagnostics.push(
                            Diagnostic::error()
                                .with_message(format!(
                                    "PROCEDUREs cannot be outside of the global scope",
                                ))
                                .with_labels(vec![Label::primary(self.fid, Span { start, end })
                                    .with_message(format!("PROCEDURE not in the global scope"))]),
                        );
                    }

                    nodes.push(Stmt::Procedure(Procedure {
                        name,
                        params: Box::new([]),
                        scope,
                    }));
                }
                Token::Keyword(Keyword::Return) => {
                    let start = self.lex.start as u32;
                    self.lex.next();
                    let ret_stmt = if self.lex.has_newline_before || self.lex.token == Token::EOF {
                        Stmt::Return {
                            start,
                            value: Expr::Void,
                        }
                    } else {
                        Stmt::Return {
                            start,
                            value: self.parse_expr(0)?,
                        }
                    };

                    if is_global_scope {
                        self.diagnostics.push(
                            Diagnostic::error()
                                .with_message(format!(
                                    "RETURN statements cannot be outside of function scopes",
                                ))
                                .with_labels(vec![Label::primary(self.fid, ret_stmt.span())
                                    .with_message(format!("RETURN not in function scope"))]),
                        );
                    }

                    self.expect_stmt_end(&ret_stmt);

                    nodes.push(ret_stmt);
                }
                Token::Keyword(Keyword::Repeat) => 'blk: {
                    self.lex.next();

                    if let Token::Keyword(Keyword::Until) = self.lex.token {
                        self.lex.next();
                        self.eat(Token::LeftParen)?;
                        let cond = self.parse_expr(0)?;
                        self.eat(Token::RightParen)?;
                        self.eat(Token::LeftBrace)?;
                        let scope = self.parse_scope(is_global_scope)?;
                        self.eat(Token::RightBrace)?;
                        nodes.push(Stmt::RepeatUntil {
                            cond: Box::new(cond),
                            scope,
                        });
                        break 'blk;
                    }

                    let n = self.parse_expr(0)?;
                    self.eat(Token::Keyword(Keyword::Times))?;
                    self.eat(Token::LeftBrace)?;
                    let scope = self.parse_scope(is_global_scope)?;
                    self.eat(Token::RightBrace)?;
                    nodes.push(Stmt::RepeatN {
                        n: Box::new(n),
                        scope,
                    });
                }
                Token::Keyword(Keyword::For) => {
                    self.lex.next();
                    self.eat(Token::Keyword(Keyword::Each))?;
                    let alias = self.eat(Token::Identifier)?;
                    self.eat(Token::Keyword(Keyword::In))?;
                    let array = self.parse_expr(0)?;
                    self.eat(Token::LeftBrace)?;
                    let scope = self.parse_scope(is_global_scope)?;
                    self.eat(Token::RightBrace)?;
                    nodes.push(Stmt::For {
                        alias,
                        array: Box::new(array),
                        scope,
                    });
                }
                _ => break,
            }
        }

        Ok(nodes.into_boxed_slice())
    }
}
