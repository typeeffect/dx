use crate::token::{Keyword, Token, TokenKind};

pub struct Lexer<'a> {
    src: &'a str,
    chars: Vec<char>,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src,
            chars: src.chars().collect(),
            pos: 0,
        }
    }

    pub fn tokenize(mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while let Some(ch) = self.peek() {
            let start = self.pos;
            match ch {
                ' ' | '\t' | '\r' => {
                    self.bump();
                }
                '\n' => {
                    self.bump();
                    tokens.push(Token::new(TokenKind::Newline, start, self.pos));
                }
                ':' => {
                    self.bump();
                    tokens.push(Token::new(TokenKind::Colon, start, self.pos));
                }
                '.' => {
                    self.bump();
                    if self.peek() == Some('.') && self.peek_n(1) == Some('.') {
                        self.bump();
                        self.bump();
                        tokens.push(Token::new(TokenKind::Ellipsis, start, self.pos));
                    } else {
                        tokens.push(Token::new(TokenKind::Dot, start, self.pos));
                    }
                }
                '\'' => {
                    self.bump();
                    tokens.push(Token::new(TokenKind::Apostrophe, start, self.pos));
                }
                '!' => {
                    self.bump();
                    tokens.push(Token::new(TokenKind::Bang, start, self.pos));
                }
                ',' => {
                    self.bump();
                    tokens.push(Token::new(TokenKind::Comma, start, self.pos));
                }
                '(' => {
                    self.bump();
                    tokens.push(Token::new(TokenKind::LParen, start, self.pos));
                }
                ')' => {
                    self.bump();
                    tokens.push(Token::new(TokenKind::RParen, start, self.pos));
                }
                '*' => {
                    self.bump();
                    tokens.push(Token::new(TokenKind::Star, start, self.pos));
                }
                '_' => {
                    self.bump();
                    tokens.push(Token::new(TokenKind::Underscore, start, self.pos));
                }
                '=' => {
                    self.bump();
                    if self.peek() == Some('>') {
                        self.bump();
                        tokens.push(Token::new(TokenKind::FatArrow, start, self.pos));
                    } else {
                        tokens.push(Token::new(TokenKind::Equal, start, self.pos));
                    }
                }
                '-' => {
                    self.bump();
                    if self.peek() == Some('>') {
                        self.bump();
                        tokens.push(Token::new(TokenKind::Arrow, start, self.pos));
                    } else {
                        tokens.push(Token::new(TokenKind::Unknown('-'), start, self.pos));
                    }
                }
                '"' => tokens.push(self.lex_string()),
                c if c.is_ascii_digit() => tokens.push(self.lex_integer()),
                c if is_ident_start(c) => tokens.push(self.lex_identifier_or_keyword()),
                other => {
                    self.bump();
                    tokens.push(Token::new(TokenKind::Unknown(other), start, self.pos));
                }
            }
        }
        tokens.push(Token::new(TokenKind::Eof, self.pos, self.pos));
        tokens
    }

    fn lex_integer(&mut self) -> Token {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.bump();
        }
        Token::new(TokenKind::Integer(self.slice(start, self.pos).to_string()), start, self.pos)
    }

    fn lex_string(&mut self) -> Token {
        let start = self.pos;
        self.bump();
        while let Some(ch) = self.peek() {
            self.bump();
            if ch == '"' {
                break;
            }
        }
        let text = if self.pos >= start + 2 {
            self.slice(start + 1, self.pos - 1).to_string()
        } else {
            String::new()
        };
        Token::new(TokenKind::String(text), start, self.pos)
    }

    fn lex_identifier_or_keyword(&mut self) -> Token {
        let start = self.pos;
        self.bump();
        while matches!(self.peek(), Some(c) if is_ident_continue(c)) {
            self.bump();
        }
        let text = self.slice(start, self.pos);
        let kind = match text {
            "fun" => TokenKind::Keyword(Keyword::Fun),
            "if" => TokenKind::Keyword(Keyword::If),
            "elif" => TokenKind::Keyword(Keyword::Elif),
            "else" => TokenKind::Keyword(Keyword::Else),
            "type" => TokenKind::Keyword(Keyword::Type),
            "lazy" => TokenKind::Keyword(Keyword::Lazy),
            "val" => TokenKind::Keyword(Keyword::Val),
            "var" => TokenKind::Keyword(Keyword::Var),
            "match" => TokenKind::Keyword(Keyword::Match),
            "from" => TokenKind::Keyword(Keyword::From),
            "import" => TokenKind::Keyword(Keyword::Import),
            "py" => TokenKind::Keyword(Keyword::Py),
            "me" => TokenKind::Keyword(Keyword::Me),
            "it" => TokenKind::Keyword(Keyword::It),
            _ => TokenKind::Identifier(text.to_string()),
        };
        Token::new(kind, start, self.pos)
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_n(&self, n: usize) -> Option<char> {
        self.chars.get(self.pos + n).copied()
    }

    fn bump(&mut self) {
        if self.pos < self.chars.len() {
            self.pos += 1;
        }
    }

    fn slice(&self, start: usize, end: usize) -> &str {
        &self.src[start..end]
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic()
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::TokenKind;

    #[test]
    fn tokenizes_member_access_and_lazy() {
        let tokens = Lexer::new("lazy me'name\n").tokenize();
        assert!(matches!(tokens[0].kind, TokenKind::Keyword(Keyword::Lazy)));
        assert!(matches!(tokens[1].kind, TokenKind::Keyword(Keyword::Me)));
        assert!(matches!(tokens[2].kind, TokenKind::Apostrophe));
        assert!(matches!(tokens[3].kind, TokenKind::Identifier(ref s) if s == "name"));
    }
}
