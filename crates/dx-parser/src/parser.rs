use crate::ast::{Item, Module};
use crate::token::{Token, TokenKind};

#[derive(Debug)]
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse_module(&mut self) -> Result<Module, ParseError> {
        let mut items = Vec::new();
        while !self.at_eof() {
            self.skip_newlines();
            if self.at_eof() {
                break;
            }
            if self.is_fun_decl_start() {
                self.consume_until_top_level_terminator();
                items.push(Item::Statement(crate::ast::Stmt::Expr(crate::ast::Expr::Name(
                    "<fun-decl-placeholder>".to_string(),
                ))));
            } else {
                self.consume_until_newline_or_eof();
            }
            self.skip_newlines();
        }
        Ok(Module { items })
    }

    fn is_fun_decl_start(&self) -> bool {
        matches!(self.peek_kind(), Some(TokenKind::Keyword(crate::token::Keyword::Fun)))
    }

    fn consume_until_top_level_terminator(&mut self) {
        while !self.at_eof() {
            if matches!(self.peek_kind(), Some(TokenKind::Dot)) {
                self.pos += 1;
                break;
            }
            self.pos += 1;
        }
    }

    fn consume_until_newline_or_eof(&mut self) {
        while !self.at_eof() {
            match self.peek_kind() {
                Some(TokenKind::Newline) | Some(TokenKind::Eof) => break,
                _ => self.pos += 1,
            }
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek_kind(), Some(TokenKind::Newline)) {
            self.pos += 1;
        }
    }

    fn at_eof(&self) -> bool {
        matches!(self.peek_kind(), Some(TokenKind::Eof) | None)
    }

    fn peek_kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.pos).map(|t| &t.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Lexer;

    #[test]
    fn parses_module_shell() {
        let src = r#"
fun fact(n: Int) -> Int:
    1
.
"#;
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        let module = parser.parse_module().expect("module should parse");
        assert_eq!(module.items.len(), 1);
    }
}
