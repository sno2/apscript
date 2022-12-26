use phf::phf_map;

use crate::ast::Span;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Token {
    EOF,
    Identifier,
    IntegerLiteral,
    BinaryLiteral,
    HexLiteral,
    FloatLiteral,
    StringLiteral,
    InvalidStringLiteral,
    /// `<-`
    ThinArrow,
    LeftParen,
    RightParen,
    LeftBrack,
    RightBrack,
    LeftBrace,
    RightBrace,
    Comma,
    Add,
    Sub,
    Mul,
    Div,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Keyword(Keyword),
}

impl AsRef<str> for Token {
    fn as_ref(&self) -> &str {
        match self {
            Self::EOF => "end of file",
            Self::Identifier => "identifier",
            Self::IntegerLiteral => "integer",
            Self::BinaryLiteral => "binary literal",
            Self::HexLiteral => "hex literal",
            Self::FloatLiteral => "float",
            Self::StringLiteral => "string",
            Self::InvalidStringLiteral => "invalid string",
            Self::ThinArrow => "`<-`",
            Self::LeftParen => "`(`",
            Self::RightParen => "`)`",
            Self::LeftBrack => "`[`",
            Self::RightBrack => "`]`",
            Self::LeftBrace => "`{`",
            Self::RightBrace => "`}`",
            Self::Comma => "`,`",
            Self::Add => "`+`",
            Self::Sub => "`-`",
            Self::Mul => "`*`",
            Self::Div => "`/`",
            Self::Equal => "`=`",
            Self::NotEqual => "`!=`",
            Self::Greater => "`>`",
            Self::GreaterEqual => "`>=`",
            Self::Less => "`<`",
            Self::LessEqual => "`<=`",
            Self::Keyword(Keyword::Mod) => "`MOD`",
            Self::Keyword(Keyword::Not) => "`NOT`",
            Self::Keyword(Keyword::And) => "`AND`",
            Self::Keyword(Keyword::Or) => "`OR`",
            Self::Keyword(Keyword::If) => "`IF`",
            Self::Keyword(Keyword::Else) => "`ELSE`",
            Self::Keyword(Keyword::Repeat) => "`REPEAT`",
            Self::Keyword(Keyword::Times) => "`TIMES`",
            Self::Keyword(Keyword::Until) => "`UNTIL`",
            Self::Keyword(Keyword::True) => "`TRUE`",
            Self::Keyword(Keyword::False) => "`FALSE`",
            Self::Keyword(Keyword::Return) => "`RETURN`",
            Self::Keyword(Keyword::For) => "`FOR`",
            Self::Keyword(Keyword::Each) => "`EACH`",
            Self::Keyword(Keyword::In) => "`IN`",
        }
    }
}

impl Token {
    pub fn lbp(self) -> u8 {
        match self {
            Self::LeftParen | Self::LeftBrack => 80,
            // Unary ops are 70
            Self::Mul | Self::Div | Self::Keyword(Keyword::Mod) => 60,
            Self::Add | Self::Sub => 50,
            Self::Less | Self::LessEqual | Self::Greater | Self::GreaterEqual => 40,
            Self::Equal | Self::NotEqual => 30,
            Self::Keyword(Keyword::And) => 20,
            Self::Keyword(Keyword::Or) => 10,
            _ => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Keyword {
    Not,
    And,
    Or,
    If,
    Else,
    Repeat,
    Times,
    Until,
    Mod,
    True,
    False,
    Return,
    For,
    Each,
    In,
}

pub static KEYWORDS: phf::Map<&'static str, Token> = phf_map! {
    "NOT" => Token::Keyword(Keyword::Not),
    "not" => Token::Keyword(Keyword::Not),
    "AND" => Token::Keyword(Keyword::And),
    "and" => Token::Keyword(Keyword::And),
    "OR" => Token::Keyword(Keyword::Or),
    "or" => Token::Keyword(Keyword::Or),
    "IF" => Token::Keyword(Keyword::If),
    "if" => Token::Keyword(Keyword::If),
    "ELSE" => Token::Keyword(Keyword::Else),
    "else" => Token::Keyword(Keyword::Else),
    "REPEAT" => Token::Keyword(Keyword::Repeat),
    "repeat" => Token::Keyword(Keyword::Repeat),
    "TIMES" => Token::Keyword(Keyword::Times),
    "times" => Token::Keyword(Keyword::Times),
    "UNTIL" => Token::Keyword(Keyword::Until),
    "until" => Token::Keyword(Keyword::Until),
    "TRUE" => Token::Keyword(Keyword::True),
    "true" => Token::Keyword(Keyword::True),
    "FALSE" => Token::Keyword(Keyword::False),
    "false" => Token::Keyword(Keyword::False),
    "RETURN" => Token::Keyword(Keyword::Return),
    "return" => Token::Keyword(Keyword::Return),
    "FOR" => Token::Keyword(Keyword::For),
    "for" => Token::Keyword(Keyword::For),
    "EACH" => Token::Keyword(Keyword::Each),
    "each" => Token::Keyword(Keyword::Each),
    "IN" => Token::Keyword(Keyword::In),
    "in" => Token::Keyword(Keyword::In),
};

#[derive(Debug)]
pub struct Lexer<'a> {
    pub start: usize,
    pub index: usize,
    pub buffer: &'a [u8],
    pub token: Token,
    pub has_newline_before: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self {
            start: 0,
            index: 0,
            buffer,
            token: Token::EOF,
            has_newline_before: false,
        }
    }

    pub fn span(&self) -> Span {
        Span {
            start: self.start as u32,
            end: self.index as u32,
        }
    }

    #[inline]
    fn integer_continue(&mut self) -> Token {
        loop {
            self.index += 1;
            match self.buffer.get(self.index) {
                Some(b'0'..=b'9') => {}
                Some(b'.') => return self.float_continue(),
                _ => break,
            }
        }

        Token::IntegerLiteral
    }

    #[inline]
    fn float_continue(&mut self) -> Token {
        loop {
            self.index += 1;
            let Some(b'0'..=b'9') = self.buffer.get(self.index) else {
				break;
			};
        }
        Token::FloatLiteral
    }

    pub fn next(&mut self) {
        self.has_newline_before = false;
        'main: loop {
            self.start = self.index;

            match self.buffer.get(self.index) {
                Some(b'\r' | b'\n') => {
                    self.has_newline_before = true;
                    loop {
                        self.index += 1;
                        let Some(b' ' | b'\t' | b'\r' | b'\n') = self.buffer.get(self.index) else {
							break
						};
                    }
                    continue 'main;
                }
                Some(b' ' | b'\t') => {
                    loop {
                        self.index += 1;
                        match self.buffer.get(self.index) {
                            Some(b' ' | b'\t') => {}
                            Some(b'\r' | b'\n') => break,
                            _ => continue 'main,
                        }
                    }

                    self.has_newline_before = true;

                    loop {
                        self.index += 1;
                        let Some(b' ' | b'\t' | b'\r' | b'\n') = self.buffer.get(self.index) else {
							break
						};
                    }

                    continue 'main;
                }
                Some(b'(') => {
                    self.index += 1;
                    self.token = Token::LeftParen;
                }
                Some(b')') => {
                    self.index += 1;
                    self.token = Token::RightParen;
                }
                Some(b'[') => {
                    self.index += 1;
                    self.token = Token::LeftBrack;
                }
                Some(b']') => {
                    self.index += 1;
                    self.token = Token::RightBrack;
                }
                Some(b'{') => {
                    self.index += 1;
                    self.token = Token::LeftBrace;
                }
                Some(b'}') => {
                    self.index += 1;
                    self.token = Token::RightBrace;
                }
                Some(b',') => {
                    self.index += 1;
                    self.token = Token::Comma;
                }
                Some(b'+') => {
                    self.index += 1;
                    self.token = Token::Add;
                }
                Some(b'-') => {
                    self.index += 1;
                    self.token = Token::Sub;
                }
                Some(b'*') => {
                    self.index += 1;
                    self.token = Token::Mul;
                }
                Some(b'/') => {
                    self.index += 1;
                    self.token = Token::Div;
                }
                Some(b'%') => {
                    self.index += 1;
                    self.token = Token::Keyword(Keyword::Mod);
                }
                Some(b'=') => {
                    self.index += 1;
                    self.token = Token::Equal;
                }
                Some(b'!') => {
                    self.index += 1;
                    self.token = if let Some(b'=') = self.buffer.get(self.index) {
                        Token::NotEqual
                    } else {
                        Token::Equal
                    };
                }
                Some(b'>') => {
                    self.index += 1;
                    self.token = if let Some(b'=') = self.buffer.get(self.index) {
                        self.index += 1;
                        Token::GreaterEqual
                    } else {
                        Token::Greater
                    };
                }
                Some(b'<') => {
                    self.index += 1;
                    self.token = match self.buffer.get(self.index) {
                        Some(b'-') => {
                            self.index += 1;
                            Token::ThinArrow
                        }
                        Some(b'=') => {
                            self.index += 1;
                            Token::LessEqual
                        }
                        _ => Token::Less,
                    };
                }
                Some(b'a'..=b'z' | b'A'..=b'Z' | b'_') => {
                    loop {
                        self.index += 1;
                        let Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_') = self.buffer.get(self.index) else {
							break;
						};
                    }

                    self.token = *KEYWORDS
                        .get(unsafe {
                            std::str::from_utf8_unchecked(&self.buffer[self.start..self.index])
                        })
                        .unwrap_or(&Token::Identifier);
                }
                Some(b'0') => {
                    self.index += 1;
                    self.token = match self.buffer.get(self.index) {
                        Some(b'x' | b'X') => {
                            loop {
                                self.index += 1;
                                let Some(b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F') = self.buffer.get(self.index) else {
									break;
								};
                            }
                            Token::HexLiteral
                        }
                        Some(b'b' | b'B') => {
                            loop {
                                self.index += 1;
                                let Some(b'0' | b'1') = self.buffer.get(self.index) else {
									break;
								};
                            }
                            Token::BinaryLiteral
                        }
                        Some(b'.') => self.float_continue(),
                        Some(b'1'..=b'9') => self.integer_continue(),
                        _ => Token::IntegerLiteral,
                    };
                }
                Some(b'"') => {
                    loop {
                        self.index += 1;
                        match self.buffer.get(self.index) {
                            Some(b'"') => {
                                self.index += 1;
                                break;
                            }
                            None => {
                                self.token = Token::InvalidStringLiteral;
                                break;
                            }
                            _ => {}
                        }
                    }
                    self.token = Token::StringLiteral;
                }
                Some(b'1'..=b'9') => self.token = self.integer_continue(),
                None => self.token = Token::EOF,
                _ => {}
            }

            break 'main;
        }
    }
}
