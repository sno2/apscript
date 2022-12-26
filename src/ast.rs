use crate::lexer::{Keyword, Token};

#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Into<std::ops::Range<usize>> for Span {
    fn into(self) -> std::ops::Range<usize> {
        self.start as usize..self.end as usize
    }
}

#[derive(Debug)]
pub enum Expr {
    Void,
    True {
        start: u32,
    },
    False {
        start: u32,
    },
    Identifier {
        span: Span,
    },
    ArrayLiteral {
        span: Span,
        values: Box<[Expr]>,
    },
    Index {
        span: Span,
        value: Box<Expr>,
        index: Box<Expr>,
    },
    StringLiteral {
        span: Span,
    },
    IntegerLiteral {
        span: Span,
    },
    FloatLiteral {
        span: Span,
    },
    BinaryLiteral {
        span: Span,
    },
    HexLiteral {
        span: Span,
    },
    FnCall {
        span: Span,
        calle: Box<Expr>,
        args: Box<[Expr]>,
    },
    UnaryOp {
        span: Span,
        kind: UnaryOpKind,
        value: Box<Expr>,
    },
    BinaryOp {
        kind: BinaryOpKind,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Paren {
        span: Span,
        value: Box<Expr>,
    },
}

pub trait Node {
    fn span(&self) -> Span;
}

impl Node for Expr {
    fn span(&self) -> Span {
        match self {
            Self::Void => unreachable!(),
            &Self::True { start } => Span {
                start,
                end: start + 4,
            },
            &Self::False { start } => Span {
                start,
                end: start + 5,
            },
            &Self::Identifier { span }
            | &Self::ArrayLiteral { span, .. }
            | &Self::Index { span, .. }
            | &Self::FnCall { span, .. }
            | &Self::UnaryOp { span, .. }
            | &Self::IntegerLiteral { span }
            | &Self::FloatLiteral { span }
            | &Self::BinaryLiteral { span }
            | &Self::StringLiteral { span }
            | &Self::HexLiteral { span }
            | &Self::Paren { span, .. } => span,
            Self::BinaryOp { lhs, rhs, .. } => Span {
                start: lhs.span().start,
                end: rhs.span().end,
            },
        }
    }
}

#[derive(Debug)]
pub enum BinaryOpKind {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}

impl From<Token> for BinaryOpKind {
    fn from(value: Token) -> Self {
        match value {
            Token::Add => Self::Add,
            Token::Sub => Self::Sub,
            Token::Mul => Self::Mul,
            Token::Div => Self::Div,
            Token::Keyword(Keyword::Mod) => Self::Mod,
            Token::Equal => Self::Equal,
            Token::NotEqual => Self::NotEqual,
            Token::Less => Self::Less,
            Token::LessEqual => Self::LessEqual,
            Token::Greater => Self::Greater,
            Token::GreaterEqual => Self::GreaterEqual,
            Token::Keyword(Keyword::And) => Self::And,
            Token::Keyword(Keyword::Or) => Self::Or,
            tok => panic!("unsupported: {tok:?}"),
        }
    }
}

#[derive(Debug)]
pub enum UnaryOpKind {
    Pos,
    Neg,
    Not,
}

#[derive(Debug)]
pub enum Stmt {
    Return {
        start: u32,
        value: Expr,
    },
    Expr(Expr),
    VarAssign {
        name: Span,
        value: Expr,
    },
    If {
        cond: Box<Expr>,
        scope: Box<[Stmt]>,
        else_ifs: Box<[ElseIf]>,
        els: Option<Box<[Stmt]>>,
    },
    RepeatN {
        n: Box<Expr>,
        scope: Box<[Stmt]>,
    },
    RepeatUntil {
        cond: Box<Expr>,
        scope: Box<[Stmt]>,
    },
    For {
        alias: Span,
        array: Box<Expr>,
        scope: Box<[Stmt]>,
    },
}

impl Node for Stmt {
    fn span(&self) -> Span {
        match self {
            Self::Return { start, value } => Span {
                start: *start,
                end: if let Expr::Void = value {
                    start + 6
                } else {
                    value.span().end
                },
            },
            Self::VarAssign { name, value } => Span {
                start: name.start,
                end: value.span().end,
            },
            _ => panic!(),
        }
    }
}

#[derive(Debug)]
pub struct ElseIf {
    pub cond: Expr,
    pub scope: Box<[Stmt]>,
}
