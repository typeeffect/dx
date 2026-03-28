#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Keyword {
    Fun,
    If,
    Elif,
    Else,
    Type,
    Lazy,
    Val,
    Var,
    Match,
    From,
    Import,
    Py,
    Me,
    It,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Identifier(String),
    Integer(String),
    String(String),
    Keyword(Keyword),
    Colon,
    Dot,
    Apostrophe,
    Arrow,
    FatArrow,
    Bang,
    Comma,
    LParen,
    RParen,
    Star,
    Plus,
    Minus,
    Lt,
    LtEq,
    Gt,
    GtEq,
    EqEq,
    Underscore,
    Equal,
    Ellipsis,
    Newline,
    Eof,
    Unknown(char),
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
