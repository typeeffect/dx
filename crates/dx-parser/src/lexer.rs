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
                '+' => {
                    self.bump();
                    tokens.push(Token::new(TokenKind::Plus, start, self.pos));
                }
                '=' => {
                    self.bump();
                    if self.peek() == Some('>') {
                        self.bump();
                        tokens.push(Token::new(TokenKind::FatArrow, start, self.pos));
                    } else if self.peek() == Some('=') {
                        self.bump();
                        tokens.push(Token::new(TokenKind::EqEq, start, self.pos));
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
                        tokens.push(Token::new(TokenKind::Minus, start, self.pos));
                    }
                }
                '<' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        tokens.push(Token::new(TokenKind::LtEq, start, self.pos));
                    } else {
                        tokens.push(Token::new(TokenKind::Lt, start, self.pos));
                    }
                }
                '>' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        tokens.push(Token::new(TokenKind::GtEq, start, self.pos));
                    } else {
                        tokens.push(Token::new(TokenKind::Gt, start, self.pos));
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
        self.bump(); // skip opening "
        let content_start = self.pos;
        while let Some(ch) = self.peek() {
            if ch == '"' {
                let content_end = self.pos;
                self.bump(); // skip closing "
                let text = self.slice(content_start, content_end).to_string();
                return Token::new(TokenKind::String(text), start, self.pos);
            }
            self.bump();
        }
        // Unterminated string: take whatever we got
        let text = self.slice(content_start, self.pos).to_string();
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
            "schema" => TokenKind::Keyword(Keyword::Schema),
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

    /// Helper: collect just the token kinds, dropping Newline and Eof for clarity.
    fn kinds(src: &str) -> Vec<TokenKind> {
        Lexer::new(src)
            .tokenize()
            .into_iter()
            .map(|t| t.kind)
            .filter(|k| !matches!(k, TokenKind::Newline | TokenKind::Eof))
            .collect()
    }

    // ── structural tokens ────────────────────────────────────────

    #[test]
    fn colon_and_dot_block_delimiters() {
        let k = kinds(": .");
        assert_eq!(k, vec![TokenKind::Colon, TokenKind::Dot]);
    }

    #[test]
    fn apostrophe_member_access() {
        let k = kinds("x'y");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("x".into()),
                TokenKind::Apostrophe,
                TokenKind::Identifier("y".into()),
            ]
        );
    }

    #[test]
    fn arrow_and_fat_arrow() {
        let k = kinds("-> =>");
        assert_eq!(k, vec![TokenKind::Arrow, TokenKind::FatArrow]);
    }

    #[test]
    fn bang_for_effects() {
        let k = kinds("!io");
        assert_eq!(
            k,
            vec![TokenKind::Bang, TokenKind::Identifier("io".into())]
        );
    }

    #[test]
    fn parens_comma_star_equal() {
        let k = kinds("(*, x) =");
        assert_eq!(
            k,
            vec![
                TokenKind::LParen,
                TokenKind::Star,
                TokenKind::Comma,
                TokenKind::Identifier("x".into()),
                TokenKind::RParen,
                TokenKind::Equal,
            ]
        );
    }

    #[test]
    fn underscore_placeholder() {
        let k = kinds("_");
        assert_eq!(k, vec![TokenKind::Underscore]);
    }

    #[test]
    fn ellipsis_three_dots() {
        let k = kinds("args: Int...");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("args".into()),
                TokenKind::Colon,
                TokenKind::Identifier("Int".into()),
                TokenKind::Ellipsis,
            ]
        );
    }

    #[test]
    fn single_dot_not_ellipsis() {
        let k = kinds(".");
        assert_eq!(k, vec![TokenKind::Dot]);
    }

    #[test]
    fn two_dots_are_two_dots() {
        let k = kinds("..");
        assert_eq!(k, vec![TokenKind::Dot, TokenKind::Dot]);
    }

    // ── keywords ─────────────────────────────────────────────────

    #[test]
    fn all_v0_1_keywords() {
        let k = kinds("schema fun if elif else type lazy val var match from import py me it");
        let expected: Vec<TokenKind> = vec![
            TokenKind::Keyword(Keyword::Schema),
            TokenKind::Keyword(Keyword::Fun),
            TokenKind::Keyword(Keyword::If),
            TokenKind::Keyword(Keyword::Elif),
            TokenKind::Keyword(Keyword::Else),
            TokenKind::Keyword(Keyword::Type),
            TokenKind::Keyword(Keyword::Lazy),
            TokenKind::Keyword(Keyword::Val),
            TokenKind::Keyword(Keyword::Var),
            TokenKind::Keyword(Keyword::Match),
            TokenKind::Keyword(Keyword::From),
            TokenKind::Keyword(Keyword::Import),
            TokenKind::Keyword(Keyword::Py),
            TokenKind::Keyword(Keyword::Me),
            TokenKind::Keyword(Keyword::It),
        ];
        assert_eq!(k, expected);
    }

    #[test]
    fn keyword_prefix_is_identifier() {
        // "funny" starts with "fun" but is an identifier, not a keyword
        let k = kinds("funny");
        assert_eq!(k, vec![TokenKind::Identifier("funny".into())]);
    }

    // ── literals ─────────────────────────────────────────────────

    #[test]
    fn integer_literal() {
        let k = kinds("42 0 999");
        assert_eq!(
            k,
            vec![
                TokenKind::Integer("42".into()),
                TokenKind::Integer("0".into()),
                TokenKind::Integer("999".into()),
            ]
        );
    }

    #[test]
    fn string_literal() {
        let k = kinds(r#""hello world""#);
        assert_eq!(k, vec![TokenKind::String("hello world".into())]);
    }

    #[test]
    fn empty_string() {
        let k = kinds(r#""""#);
        assert_eq!(k, vec![TokenKind::String("".into())]);
    }

    #[test]
    fn unterminated_string() {
        let k = kinds(r#""oops"#);
        assert_eq!(k, vec![TokenKind::String("oops".into())]);
    }

    // ── identifiers ──────────────────────────────────────────────

    #[test]
    fn identifier_with_underscores() {
        let k = kinds("read_csv my_var x1");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("read_csv".into()),
                TokenKind::Identifier("my_var".into()),
                TokenKind::Identifier("x1".into()),
            ]
        );
    }

    #[test]
    fn underscore_before_ident_is_two_tokens() {
        // standalone _ followed by identifier
        let k = kinds("_name");
        assert_eq!(
            k,
            vec![
                TokenKind::Underscore,
                TokenKind::Identifier("name".into()),
            ]
        );
    }

    // ── member access chains ─────────────────────────────────────

    #[test]
    fn member_access_chain() {
        let k = kinds("orders'filter'first'name");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("orders".into()),
                TokenKind::Apostrophe,
                TokenKind::Identifier("filter".into()),
                TokenKind::Apostrophe,
                TokenKind::Identifier("first".into()),
                TokenKind::Apostrophe,
                TokenKind::Identifier("name".into()),
            ]
        );
    }

    #[test]
    fn member_access_with_call() {
        let k = kinds("users'filter(x)");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("users".into()),
                TokenKind::Apostrophe,
                TokenKind::Identifier("filter".into()),
                TokenKind::LParen,
                TokenKind::Identifier("x".into()),
                TokenKind::RParen,
            ]
        );
    }

    #[test]
    fn placeholder_member_access() {
        // _'email  =>  Underscore Apostrophe Identifier("email")
        let k = kinds("_'email");
        assert_eq!(
            k,
            vec![
                TokenKind::Underscore,
                TokenKind::Apostrophe,
                TokenKind::Identifier("email".into()),
            ]
        );
    }

    // ── lazy ─────────────────────────────────────────────────────

    #[test]
    fn lazy_expression() {
        let k = kinds("lazy me'name");
        assert_eq!(
            k,
            vec![
                TokenKind::Keyword(Keyword::Lazy),
                TokenKind::Keyword(Keyword::Me),
                TokenKind::Apostrophe,
                TokenKind::Identifier("name".into()),
            ]
        );
    }

    #[test]
    fn lazy_block() {
        let k = kinds("lazy:\n  x\n.");
        assert_eq!(
            k,
            vec![
                TokenKind::Keyword(Keyword::Lazy),
                TokenKind::Colon,
                TokenKind::Identifier("x".into()),
                TokenKind::Dot,
            ]
        );
    }

    // ── lazy parameter types (lazy T desugars to () -> T) ─────────

    #[test]
    fn lazy_param_type_plain() {
        // msg: lazy Str
        let k = kinds("msg: lazy Str");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("msg".into()),
                TokenKind::Colon,
                TokenKind::Keyword(Keyword::Lazy),
                TokenKind::Identifier("Str".into()),
            ]
        );
    }

    #[test]
    fn lazy_param_type_with_effects() {
        // compute: lazy Value !io
        let k = kinds("compute: lazy Value !io");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("compute".into()),
                TokenKind::Colon,
                TokenKind::Keyword(Keyword::Lazy),
                TokenKind::Identifier("Value".into()),
                TokenKind::Bang,
                TokenKind::Identifier("io".into()),
            ]
        );
    }

    #[test]
    fn lazy_param_type_multiple_effects() {
        // msg: lazy Str !io !throw
        let k = kinds("msg: lazy Str !io !throw");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("msg".into()),
                TokenKind::Colon,
                TokenKind::Keyword(Keyword::Lazy),
                TokenKind::Identifier("Str".into()),
                TokenKind::Bang,
                TokenKind::Identifier("io".into()),
                TokenKind::Bang,
                TokenKind::Identifier("throw".into()),
            ]
        );
    }

    #[test]
    fn lazy_param_type_generic() {
        // fallback: lazy T
        let k = kinds("fallback: lazy T");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("fallback".into()),
                TokenKind::Colon,
                TokenKind::Keyword(Keyword::Lazy),
                TokenKind::Identifier("T".into()),
            ]
        );
    }

    #[test]
    fn function_with_lazy_param() {
        // fun log(msg: lazy Str !io) -> Unit:
        let k = kinds("fun log(msg: lazy Str !io) -> Unit:");
        assert_eq!(
            k,
            vec![
                TokenKind::Keyword(Keyword::Fun),
                TokenKind::Identifier("log".into()),
                TokenKind::LParen,
                TokenKind::Identifier("msg".into()),
                TokenKind::Colon,
                TokenKind::Keyword(Keyword::Lazy),
                TokenKind::Identifier("Str".into()),
                TokenKind::Bang,
                TokenKind::Identifier("io".into()),
                TokenKind::RParen,
                TokenKind::Arrow,
                TokenKind::Identifier("Unit".into()),
                TokenKind::Colon,
            ]
        );
    }

    // ── function signatures with effects ─────────────────────────

    #[test]
    fn function_signature_with_effects() {
        let k = kinds("fun read(path: Str) -> Str !io !throw:");
        assert_eq!(
            k,
            vec![
                TokenKind::Keyword(Keyword::Fun),
                TokenKind::Identifier("read".into()),
                TokenKind::LParen,
                TokenKind::Identifier("path".into()),
                TokenKind::Colon,
                TokenKind::Identifier("Str".into()),
                TokenKind::RParen,
                TokenKind::Arrow,
                TokenKind::Identifier("Str".into()),
                TokenKind::Bang,
                TokenKind::Identifier("io".into()),
                TokenKind::Bang,
                TokenKind::Identifier("throw".into()),
                TokenKind::Colon,
            ]
        );
    }

    #[test]
    fn function_with_multiple_params_and_return() {
        let k = kinds("fun add(a: Int, b: Int) -> Int:");
        assert_eq!(
            k,
            vec![
                TokenKind::Keyword(Keyword::Fun),
                TokenKind::Identifier("add".into()),
                TokenKind::LParen,
                TokenKind::Identifier("a".into()),
                TokenKind::Colon,
                TokenKind::Identifier("Int".into()),
                TokenKind::Comma,
                TokenKind::Identifier("b".into()),
                TokenKind::Colon,
                TokenKind::Identifier("Int".into()),
                TokenKind::RParen,
                TokenKind::Arrow,
                TokenKind::Identifier("Int".into()),
                TokenKind::Colon,
            ]
        );
    }

    #[test]
    fn function_with_py_effect() {
        let k = kinds("fun load(path: Str) -> PyObj !py !throw:");
        assert_eq!(
            k,
            vec![
                TokenKind::Keyword(Keyword::Fun),
                TokenKind::Identifier("load".into()),
                TokenKind::LParen,
                TokenKind::Identifier("path".into()),
                TokenKind::Colon,
                TokenKind::Identifier("Str".into()),
                TokenKind::RParen,
                TokenKind::Arrow,
                TokenKind::Identifier("PyObj".into()),
                TokenKind::Bang,
                TokenKind::Keyword(Keyword::Py),
                TokenKind::Bang,
                TokenKind::Identifier("throw".into()),
                TokenKind::Colon,
            ]
        );
    }

    // ── from py ... import ... ───────────────────────────────────

    #[test]
    fn from_py_import() {
        let k = kinds("from py pandas import read_csv");
        assert_eq!(
            k,
            vec![
                TokenKind::Keyword(Keyword::From),
                TokenKind::Keyword(Keyword::Py),
                TokenKind::Identifier("pandas".into()),
                TokenKind::Keyword(Keyword::Import),
                TokenKind::Identifier("read_csv".into()),
            ]
        );
    }

    #[test]
    fn from_py_import_multiple() {
        let k = kinds("from py builtins import print, len");
        assert_eq!(
            k,
            vec![
                TokenKind::Keyword(Keyword::From),
                TokenKind::Keyword(Keyword::Py),
                TokenKind::Identifier("builtins".into()),
                TokenKind::Keyword(Keyword::Import),
                TokenKind::Identifier("print".into()),
                TokenKind::Comma,
                TokenKind::Identifier("len".into()),
            ]
        );
    }

    // ── named arguments ──────────────────────────────────────────

    #[test]
    fn named_arguments_in_call() {
        let k = kinds("foo(x: 1, name: \"hi\")");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("foo".into()),
                TokenKind::LParen,
                TokenKind::Identifier("x".into()),
                TokenKind::Colon,
                TokenKind::Integer("1".into()),
                TokenKind::Comma,
                TokenKind::Identifier("name".into()),
                TokenKind::Colon,
                TokenKind::String("hi".into()),
                TokenKind::RParen,
            ]
        );
    }

    #[test]
    fn mixed_positional_and_named() {
        let k = kinds("bar(1, key: 2)");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("bar".into()),
                TokenKind::LParen,
                TokenKind::Integer("1".into()),
                TokenKind::Comma,
                TokenKind::Identifier("key".into()),
                TokenKind::Colon,
                TokenKind::Integer("2".into()),
                TokenKind::RParen,
            ]
        );
    }

    // ── lambda expressions ───────────────────────────────────────

    #[test]
    fn simple_lambda() {
        let k = kinds("x => x");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("x".into()),
                TokenKind::FatArrow,
                TokenKind::Identifier("x".into()),
            ]
        );
    }

    #[test]
    fn typed_lambda() {
        let k = kinds("(x: Int) => x");
        assert_eq!(
            k,
            vec![
                TokenKind::LParen,
                TokenKind::Identifier("x".into()),
                TokenKind::Colon,
                TokenKind::Identifier("Int".into()),
                TokenKind::RParen,
                TokenKind::FatArrow,
                TokenKind::Identifier("x".into()),
            ]
        );
    }

    #[test]
    fn block_lambda() {
        let k = kinds("x =>:\n  x\n.");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("x".into()),
                TokenKind::FatArrow,
                TokenKind::Colon,
                TokenKind::Identifier("x".into()),
                TokenKind::Dot,
            ]
        );
    }

    // ── val / var / rebind ───────────────────────────────────────

    #[test]
    fn val_and_var_bindings() {
        let k = kinds("val x = 1\nvar y = 2\ny = 3");
        assert_eq!(
            k,
            vec![
                TokenKind::Keyword(Keyword::Val),
                TokenKind::Identifier("x".into()),
                TokenKind::Equal,
                TokenKind::Integer("1".into()),
                // newline filtered out
                TokenKind::Keyword(Keyword::Var),
                TokenKind::Identifier("y".into()),
                TokenKind::Equal,
                TokenKind::Integer("2".into()),
                // newline filtered out
                TokenKind::Identifier("y".into()),
                TokenKind::Equal,
                TokenKind::Integer("3".into()),
            ]
        );
    }

    // ── match expression ─────────────────────────────────────────

    #[test]
    fn match_expression() {
        let k = kinds("match x:\n  y: y\n.");
        assert_eq!(
            k,
            vec![
                TokenKind::Keyword(Keyword::Match),
                TokenKind::Identifier("x".into()),
                TokenKind::Colon,
                TokenKind::Identifier("y".into()),
                TokenKind::Colon,
                TokenKind::Identifier("y".into()),
                TokenKind::Dot,
            ]
        );
    }

    // ── if / elif / else ─────────────────────────────────────────

    #[test]
    fn if_elif_else() {
        let k = kinds("if x: a. elif y: b. else: c.");
        // Note: this is a flat sequence for the lexer
        assert!(k.contains(&TokenKind::Keyword(Keyword::If)));
        assert!(k.contains(&TokenKind::Keyword(Keyword::Elif)));
        assert!(k.contains(&TokenKind::Keyword(Keyword::Else)));
    }

    // ── variadic parameter ───────────────────────────────────────

    #[test]
    fn variadic_param_with_ellipsis() {
        let k = kinds("fun f(args: Int...):");
        assert_eq!(
            k,
            vec![
                TokenKind::Keyword(Keyword::Fun),
                TokenKind::Identifier("f".into()),
                TokenKind::LParen,
                TokenKind::Identifier("args".into()),
                TokenKind::Colon,
                TokenKind::Identifier("Int".into()),
                TokenKind::Ellipsis,
                TokenKind::RParen,
                TokenKind::Colon,
            ]
        );
    }

    // ── star parameter (keyword-only separator) ──────────────────

    #[test]
    fn star_param_separator() {
        let k = kinds("fun f(a: Int, *, key: Str):");
        assert_eq!(
            k,
            vec![
                TokenKind::Keyword(Keyword::Fun),
                TokenKind::Identifier("f".into()),
                TokenKind::LParen,
                TokenKind::Identifier("a".into()),
                TokenKind::Colon,
                TokenKind::Identifier("Int".into()),
                TokenKind::Comma,
                TokenKind::Star,
                TokenKind::Comma,
                TokenKind::Identifier("key".into()),
                TokenKind::Colon,
                TokenKind::Identifier("Str".into()),
                TokenKind::RParen,
                TokenKind::Colon,
            ]
        );
    }

    // ── default parameter values ─────────────────────────────────

    #[test]
    fn default_param_value() {
        let k = kinds("fun f(x: Int = 0):");
        assert_eq!(
            k,
            vec![
                TokenKind::Keyword(Keyword::Fun),
                TokenKind::Identifier("f".into()),
                TokenKind::LParen,
                TokenKind::Identifier("x".into()),
                TokenKind::Colon,
                TokenKind::Identifier("Int".into()),
                TokenKind::Equal,
                TokenKind::Integer("0".into()),
                TokenKind::RParen,
                TokenKind::Colon,
            ]
        );
    }

    // ── trailing closure syntax ──────────────────────────────────

    #[test]
    fn trailing_closure() {
        let k = kinds("orders'filter:\n  x => x'total\n.");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("orders".into()),
                TokenKind::Apostrophe,
                TokenKind::Identifier("filter".into()),
                TokenKind::Colon,
                TokenKind::Identifier("x".into()),
                TokenKind::FatArrow,
                TokenKind::Identifier("x".into()),
                TokenKind::Apostrophe,
                TokenKind::Identifier("total".into()),
                TokenKind::Dot,
            ]
        );
    }

    // ── spans ────────────────────────────────────────────────────

    #[test]
    fn token_spans_are_correct() {
        let tokens = Lexer::new("val x = 42").tokenize();
        // "val" at 0..3
        assert_eq!(tokens[0].start, 0);
        assert_eq!(tokens[0].end, 3);
        // "x" at 4..5
        assert_eq!(tokens[1].start, 4);
        assert_eq!(tokens[1].end, 5);
        // "=" at 6..7
        assert_eq!(tokens[2].start, 6);
        assert_eq!(tokens[2].end, 7);
        // "42" at 8..10
        assert_eq!(tokens[3].start, 8);
        assert_eq!(tokens[3].end, 10);
    }

    #[test]
    fn string_span_includes_quotes() {
        let tokens = Lexer::new(r#""hi""#).tokenize();
        assert_eq!(tokens[0].start, 0);
        assert_eq!(tokens[0].end, 4); // includes both quotes
        assert!(matches!(tokens[0].kind, TokenKind::String(ref s) if s == "hi"));
    }

    // ── whitespace handling ──────────────────────────────────────

    #[test]
    fn whitespace_is_skipped() {
        let k = kinds("  x  \t  y  ");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("x".into()),
                TokenKind::Identifier("y".into()),
            ]
        );
    }

    #[test]
    fn newlines_are_emitted() {
        let tokens: Vec<TokenKind> = Lexer::new("x\ny")
            .tokenize()
            .into_iter()
            .map(|t| t.kind)
            .filter(|k| !matches!(k, TokenKind::Eof))
            .collect();
        assert_eq!(
            tokens,
            vec![
                TokenKind::Identifier("x".into()),
                TokenKind::Newline,
                TokenKind::Identifier("y".into()),
            ]
        );
    }

    // ── unknown characters ───────────────────────────────────────

    #[test]
    fn unknown_chars_are_captured() {
        let k = kinds("@");
        assert_eq!(k, vec![TokenKind::Unknown('@')]);
    }

    #[test]
    fn minus_standalone() {
        let k = kinds("-");
        assert_eq!(k, vec![TokenKind::Minus]);
    }

    #[test]
    fn gt_standalone() {
        let k = kinds(">");
        assert_eq!(k, vec![TokenKind::Gt]);
    }

    #[test]
    fn operator_gt_eq() {
        let k = kinds("x >= 0");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("x".into()),
                TokenKind::GtEq,
                TokenKind::Integer("0".into()),
            ]
        );
    }

    // ── operator tokens ──────────────────────────────────────────

    #[test]
    fn operator_plus() {
        let k = kinds("x + 1");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("x".into()),
                TokenKind::Plus,
                TokenKind::Integer("1".into()),
            ]
        );
    }

    #[test]
    fn operator_minus() {
        let k = kinds("n - 1");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("n".into()),
                TokenKind::Minus,
                TokenKind::Integer("1".into()),
            ]
        );
    }

    #[test]
    fn operator_star_in_expr() {
        let k = kinds("n * 2");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("n".into()),
                TokenKind::Star,
                TokenKind::Integer("2".into()),
            ]
        );
    }

    #[test]
    fn operator_lt() {
        let k = kinds("x < 0");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("x".into()),
                TokenKind::Lt,
                TokenKind::Integer("0".into()),
            ]
        );
    }

    #[test]
    fn operator_lt_eq() {
        let k = kinds("n <= 1");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("n".into()),
                TokenKind::LtEq,
                TokenKind::Integer("1".into()),
            ]
        );
    }

    #[test]
    fn operator_eq_eq() {
        let k = kinds("x == 0");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("x".into()),
                TokenKind::EqEq,
                TokenKind::Integer("0".into()),
            ]
        );
    }

    #[test]
    fn eq_eq_vs_fat_arrow() {
        // ==> should be == then >
        let k = kinds("== =>");
        assert_eq!(k, vec![TokenKind::EqEq, TokenKind::FatArrow]);
    }

    // ── complete program ─────────────────────────────────────────

    #[test]
    fn full_program_tokenization() {
        let src = r#"from py pandas import read_csv

fun load(path: Str) -> PyObj !py !throw:
    val frame = read_csv(path)
    frame
.

fun demo() -> Unit:
    val f = _'email
    val g = lazy me'name
    users'filter(x => x'active)
.
"#;
        let tokens = Lexer::new(src).tokenize();
        // Should not contain any Unknown tokens
        for t in &tokens {
            assert!(
                !matches!(t.kind, TokenKind::Unknown(_)),
                "unexpected Unknown token: {:?}",
                t
            );
        }
        // Should end with Eof
        assert!(matches!(tokens.last().unwrap().kind, TokenKind::Eof));
    }
}
