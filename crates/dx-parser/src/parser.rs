use crate::ast::{
    Arg, Expr, FunctionDecl, ImportPyDecl, Item, LambdaBody, LambdaParam, MatchArm, Module,
    Param, Pattern, Stmt, TypeExpr,
};
use crate::token::{Keyword, Token, TokenKind};

#[derive(Debug)]
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
}

impl ParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse_module(&mut self) -> Result<Module, ParseError> {
        let mut items = Vec::new();
        self.skip_newlines();
        while !self.at_eof() {
            items.push(self.parse_item()?);
            self.skip_newlines();
        }
        Ok(Module { items })
    }

    fn parse_item(&mut self) -> Result<Item, ParseError> {
        self.skip_newlines();
        if self.at_keyword(Keyword::From) {
            return Ok(Item::ImportPy(self.parse_import_py()?));
        }
        if self.at_keyword(Keyword::Fun) {
            return Ok(Item::Function(self.parse_function_decl()?));
        }
        Ok(Item::Statement(self.parse_stmt()?))
    }

    fn parse_import_py(&mut self) -> Result<ImportPyDecl, ParseError> {
        self.expect_keyword(Keyword::From)?;
        self.expect_keyword(Keyword::Py)?;
        let module = self.expect_identifier()?;
        self.expect_keyword(Keyword::Import)?;
        let mut names = vec![self.expect_identifier()?];
        while self.at(TokenKind::Comma) {
            self.bump();
            names.push(self.expect_identifier()?);
        }
        self.consume_optional_newline();
        Ok(ImportPyDecl { module, names })
    }

    fn parse_function_decl(&mut self) -> Result<FunctionDecl, ParseError> {
        self.expect_keyword(Keyword::Fun)?;
        let name = self.expect_identifier()?;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(TokenKind::RParen)?;
        let return_type = if self.at(TokenKind::Arrow) {
            self.bump();
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        let effects = self.parse_effects()?;
        self.expect(TokenKind::Colon)?;
        let body = self.parse_block_until_dot()?;
        Ok(FunctionDecl {
            name,
            params,
            return_type,
            effects,
            body,
        })
    }

    fn parse_param_list(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();
        self.skip_newlines();
        if self.at(TokenKind::RParen) {
            return Ok(params);
        }
        loop {
            let name = self.expect_identifier()?;
            self.expect(TokenKind::Colon)?;
            let ty = self.parse_type_expr()?;
            params.push(Param { name, ty });
            if !self.at(TokenKind::Comma) {
                break;
            }
            self.bump();
        }
        Ok(params)
    }

    fn parse_type_expr(&mut self) -> Result<TypeExpr, ParseError> {
        if self.at_keyword(Keyword::Lazy) {
            self.bump();
            let ret = Box::new(self.parse_type_expr()?);
            let effects = self.parse_effects()?;
            return Ok(TypeExpr::Function {
                params: Vec::new(),
                ret,
                effects,
            });
        }
        if self.at(TokenKind::LParen) {
            self.bump();
            let mut params = Vec::new();
            if !self.at(TokenKind::RParen) {
                loop {
                    params.push(self.parse_type_expr()?);
                    if !self.at(TokenKind::Comma) {
                        break;
                    }
                    self.bump();
                }
            }
            self.expect(TokenKind::RParen)?;
            self.expect(TokenKind::Arrow)?;
            let ret = Box::new(self.parse_type_expr()?);
            let effects = self.parse_effects()?;
            return Ok(TypeExpr::Function {
                params,
                ret,
                effects,
            });
        }
        Ok(TypeExpr::Name(self.expect_identifier()?))
    }

    fn parse_effects(&mut self) -> Result<Vec<String>, ParseError> {
        let mut effects = Vec::new();
        while self.at(TokenKind::Bang) {
            self.bump();
            effects.push(self.expect_effect_name()?);
        }
        Ok(effects)
    }

    fn parse_block_until_dot(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut body = Vec::new();
        self.skip_newlines();
        while !self.at(TokenKind::Dot) {
            if self.at_eof() {
                return Err(ParseError::new("unterminated block"));
            }
            body.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        self.expect(TokenKind::Dot)?;
        self.consume_optional_newline();
        Ok(body)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.skip_newlines();
        if self.at_keyword(Keyword::Val) {
            self.bump();
            let name = self.expect_identifier()?;
            self.expect(TokenKind::Equal)?;
            let value = self.parse_expr()?;
            self.consume_optional_newline();
            return Ok(Stmt::ValBind { name, value });
        }
        if self.at_keyword(Keyword::Var) {
            self.bump();
            let name = self.expect_identifier()?;
            self.expect(TokenKind::Equal)?;
            let value = self.parse_expr()?;
            self.consume_optional_newline();
            return Ok(Stmt::VarBind { name, value });
        }

        let expr = self.parse_expr()?;
        if self.at(TokenKind::Equal) {
            self.bump();
            let value = self.parse_expr()?;
            self.consume_optional_newline();
            if let Expr::Name(name) = expr {
                return Ok(Stmt::Rebind { name, value });
            }
            return Err(ParseError::new("left-hand side of assignment must be a name"));
        }
        self.consume_optional_newline();
        Ok(Stmt::Expr(expr))
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_lambda_or_postfix()
    }

    fn parse_lambda_or_postfix(&mut self) -> Result<Expr, ParseError> {
        if self.at_keyword(Keyword::Lazy) {
            return self.parse_lazy_expr();
        }

        if self.at_keyword(Keyword::If) {
            return self.parse_if_expr();
        }

        if self.at_keyword(Keyword::Match) {
            return self.parse_match_expr();
        }

        if let Some(expr) = self.try_parse_lambda()? {
            return Ok(expr);
        }

        self.parse_postfix_expr()
    }

    fn parse_if_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect_keyword(Keyword::If)?;
        let cond = self.parse_expr()?;
        self.expect(TokenKind::Colon)?;
        let then_body = self.parse_block_stmts()?;

        let mut branches = vec![(cond, then_body)];

        while self.at_keyword(Keyword::Elif) {
            self.bump();
            let elif_cond = self.parse_expr()?;
            self.expect(TokenKind::Colon)?;
            let elif_body = self.parse_block_stmts()?;
            branches.push((elif_cond, elif_body));
        }

        let else_branch = if self.at_keyword(Keyword::Else) {
            self.bump();
            self.expect(TokenKind::Colon)?;
            Some(self.parse_block_stmts()?)
        } else {
            None
        };

        self.expect(TokenKind::Dot)?;
        self.consume_optional_newline();

        Ok(Expr::If {
            branches,
            else_branch,
        })
    }

    /// Parse statements inside a block until we hit `.`, `elif`, or `else`.
    /// Does NOT consume the terminator.
    fn parse_block_stmts(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut body = Vec::new();
        self.skip_newlines();
        while !self.at(TokenKind::Dot)
            && !self.at_keyword(Keyword::Elif)
            && !self.at_keyword(Keyword::Else)
        {
            if self.at_eof() {
                return Err(ParseError::new("unterminated block"));
            }
            body.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        Ok(body)
    }

    fn parse_match_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect_keyword(Keyword::Match)?;
        let scrutinee = self.parse_expr()?;
        self.expect(TokenKind::Colon)?;

        let mut arms = Vec::new();
        self.skip_newlines();
        while !self.at(TokenKind::Dot) {
            if self.at_eof() {
                return Err(ParseError::new("unterminated match expression"));
            }
            let pattern = self.parse_pattern()?;
            self.expect(TokenKind::Colon)?;
            let body = self.parse_match_arm_body()?;
            arms.push(MatchArm { pattern, body });
            self.skip_newlines();
        }

        self.expect(TokenKind::Dot)?;
        self.consume_optional_newline();

        Ok(Expr::Match {
            scrutinee: Box::new(scrutinee),
            arms,
        })
    }

    fn parse_match_arm_body(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut body = Vec::new();
        self.skip_newlines();
        while !self.at(TokenKind::Dot) && !self.starts_match_arm() {
            if self.at_eof() {
                return Err(ParseError::new("unterminated match arm"));
            }
            body.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        Ok(body)
    }

    fn try_parse_lambda(&mut self) -> Result<Option<Expr>, ParseError> {
        let checkpoint = self.pos;

        if let Some(single) = self.try_parse_single_param_lambda()? {
            return Ok(Some(single));
        }

        self.pos = checkpoint;
        if !self.at(TokenKind::LParen) {
            return Ok(None);
        }
        self.bump();
        let mut params = Vec::new();
        if !self.at(TokenKind::RParen) {
            loop {
                let name = self.expect_identifier()?;
                let ty = if self.at(TokenKind::Colon) {
                    self.bump();
                    Some(self.parse_type_expr()?)
                } else {
                    None
                };
                params.push(LambdaParam { name, ty });
                if !self.at(TokenKind::Comma) {
                    break;
                }
                self.bump();
            }
        }
        self.expect(TokenKind::RParen)?;
        if !self.at(TokenKind::FatArrow) {
            self.pos = checkpoint;
            return Ok(None);
        }
        self.bump();
        let body = self.parse_lambda_body()?;
        Ok(Some(Expr::Lambda { params, body }))
    }

    fn try_parse_single_param_lambda(&mut self) -> Result<Option<Expr>, ParseError> {
        let checkpoint = self.pos;
        let name = match self.peek_kind() {
            Some(TokenKind::Identifier(name)) => name.clone(),
            _ => return Ok(None),
        };
        self.bump();
        if !self.at(TokenKind::FatArrow) {
            self.pos = checkpoint;
            return Ok(None);
        }
        self.bump();
        let body = self.parse_lambda_body()?;
        Ok(Some(Expr::Lambda {
            params: vec![LambdaParam { name, ty: None }],
            body,
        }))
    }

    fn parse_lambda_body(&mut self) -> Result<LambdaBody, ParseError> {
        if self.at(TokenKind::Colon) {
            self.bump();
            let block = self.parse_block_until_dot()?;
            Ok(LambdaBody::Block(block))
        } else {
            Ok(LambdaBody::Expr(Box::new(self.parse_expr()?)))
        }
    }

    fn parse_lazy_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect_keyword(Keyword::Lazy)?;
        let body = if self.at(TokenKind::Colon) {
            self.bump();
            LambdaBody::Block(self.parse_block_until_dot()?)
        } else {
            LambdaBody::Expr(Box::new(self.parse_expr()?))
        };
        Ok(Expr::Lazy { body })
    }

    fn parse_postfix_expr(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary_expr()?;
        loop {
            if self.at(TokenKind::Apostrophe) {
                self.bump();
                let name = self.expect_identifier()?;
                expr = Expr::Member {
                    base: Box::new(expr),
                    name,
                };
                continue;
            }
            if self.at(TokenKind::LParen) {
                self.bump();
                let args = self.parse_arg_list()?;
                self.expect(TokenKind::RParen)?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                };
                continue;
            }
            break;
        }
        Ok(expr)
    }

    fn parse_arg_list(&mut self) -> Result<Vec<Arg>, ParseError> {
        let mut args = Vec::new();
        if self.at(TokenKind::RParen) {
            return Ok(args);
        }
        loop {
            let checkpoint = self.pos;
            if let Some(TokenKind::Identifier(name)) = self.peek_kind() {
                let name = name.clone();
                self.bump();
                if self.at(TokenKind::Colon) {
                    self.bump();
                    let value = self.parse_expr()?;
                    args.push(Arg::Named { name, value });
                } else {
                    self.pos = checkpoint;
                    args.push(Arg::Positional(self.parse_expr()?));
                }
            } else {
                args.push(Arg::Positional(self.parse_expr()?));
            }
            if !self.at(TokenKind::Comma) {
                break;
            }
            self.bump();
        }
        Ok(args)
    }

    fn parse_primary_expr(&mut self) -> Result<Expr, ParseError> {
        match self.peek_kind().cloned() {
            Some(TokenKind::Identifier(name)) => {
                self.bump();
                Ok(Expr::Name(name))
            }
            Some(TokenKind::Keyword(Keyword::Me)) => {
                self.bump();
                Ok(Expr::Name("me".to_string()))
            }
            Some(TokenKind::Keyword(Keyword::It)) => {
                self.bump();
                Ok(Expr::Name("it".to_string()))
            }
            Some(TokenKind::Integer(value)) => {
                self.bump();
                Ok(Expr::Integer(value))
            }
            Some(TokenKind::String(value)) => {
                self.bump();
                Ok(Expr::String(value))
            }
            Some(TokenKind::Underscore) => {
                self.bump();
                Ok(Expr::Placeholder)
            }
            Some(TokenKind::LParen) => {
                self.bump();
                let expr = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            other => Err(ParseError::new(format!(
                "unexpected token in expression: {:?}",
                other
            ))),
        }
    }

    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        match self.peek_kind().cloned() {
            Some(TokenKind::Underscore) => {
                self.bump();
                Ok(Pattern::Wildcard)
            }
            Some(TokenKind::Identifier(name)) => {
                self.bump();
                if self.at(TokenKind::LParen) {
                    self.bump();
                    let mut args = Vec::new();
                    if !self.at(TokenKind::RParen) {
                        loop {
                            args.push(self.parse_pattern()?);
                            if !self.at(TokenKind::Comma) {
                                break;
                            }
                            self.bump();
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    Ok(Pattern::Constructor { name, args })
                } else {
                    Ok(Pattern::Name(name))
                }
            }
            other => Err(ParseError::new(format!(
                "unexpected token in pattern: {:?}",
                other
            ))),
        }
    }

    fn consume_optional_newline(&mut self) {
        if self.at(TokenKind::Newline) {
            self.bump();
        }
        self.skip_newlines();
    }

    fn skip_newlines(&mut self) {
        while self.at(TokenKind::Newline) {
            self.bump();
        }
    }

    fn at_keyword(&self, keyword: Keyword) -> bool {
        matches!(self.peek_kind(), Some(TokenKind::Keyword(k)) if *k == keyword)
    }

    fn at(&self, kind: TokenKind) -> bool {
        matches!(self.peek_kind(), Some(k) if *k == kind)
    }

    fn at_eof(&self) -> bool {
        matches!(self.peek_kind(), Some(TokenKind::Eof) | None)
    }

    fn starts_match_arm(&self) -> bool {
        let mut pos = self.pos;
        while matches!(
            self.tokens.get(pos).map(|t| &t.kind),
            Some(TokenKind::Newline)
        ) {
            pos += 1;
        }

        match self.tokens.get(pos).map(|t| &t.kind) {
            Some(TokenKind::Underscore) => matches!(
                self.tokens.get(pos + 1).map(|t| &t.kind),
                Some(TokenKind::Colon)
            ),
            Some(TokenKind::Identifier(_)) => match self.tokens.get(pos + 1).map(|t| &t.kind) {
                Some(TokenKind::Colon) => true,
                Some(TokenKind::LParen) => {
                    let mut depth = 1usize;
                    let mut i = pos + 2;
                    while let Some(kind) = self.tokens.get(i).map(|t| &t.kind) {
                        match kind {
                            TokenKind::LParen => depth += 1,
                            TokenKind::RParen => {
                                depth -= 1;
                                if depth == 0 {
                                    return matches!(
                                        self.tokens.get(i + 1).map(|t| &t.kind),
                                        Some(TokenKind::Colon)
                                    );
                                }
                            }
                            _ => {}
                        }
                        i += 1;
                    }
                    false
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn expect_keyword(&mut self, keyword: Keyword) -> Result<(), ParseError> {
        if self.at_keyword(keyword.clone()) {
            self.bump();
            Ok(())
        } else {
            Err(ParseError::new(format!("expected keyword {:?}", keyword)))
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<(), ParseError> {
        if self.at(kind.clone()) {
            self.bump();
            Ok(())
        } else {
            Err(ParseError::new(format!("expected token {:?}", kind)))
        }
    }

    fn expect_identifier(&mut self) -> Result<String, ParseError> {
        match self.peek_kind().cloned() {
            Some(TokenKind::Identifier(name)) => {
                self.bump();
                Ok(name)
            }
            other => Err(ParseError::new(format!(
                "expected identifier, found {:?}",
                other
            ))),
        }
    }

    fn expect_effect_name(&mut self) -> Result<String, ParseError> {
        match self.peek_kind().cloned() {
            Some(TokenKind::Identifier(name)) => {
                self.bump();
                Ok(name)
            }
            Some(TokenKind::Keyword(Keyword::Py)) => {
                self.bump();
                Ok("py".to_string())
            }
            other => Err(ParseError::new(format!(
                "expected effect name, found {:?}",
                other
            ))),
        }
    }

    fn bump(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
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
    fn parses_import_and_function() {
        let src = r#"
from py pandas import read_csv

fun load(path: Str) -> PyObj !py !throw:
    val frame = read_csv(path)
    frame
.
"#;
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        let module = parser.parse_module().expect("module should parse");
        assert_eq!(module.items.len(), 2);
        match &module.items[0] {
            Item::ImportPy(import) => {
                assert_eq!(import.module, "pandas");
                assert_eq!(import.names, vec!["read_csv"]);
            }
            other => panic!("expected import, got {other:?}"),
        }
        match &module.items[1] {
            Item::Function(function) => {
                assert_eq!(function.name, "load");
                assert_eq!(function.effects, vec!["py", "throw"]);
                assert_eq!(function.params.len(), 1);
                assert_eq!(function.body.len(), 2);
            }
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn parses_member_access_lambda_and_lazy() {
        let src = r#"
fun demo() -> Unit:
    val f = _'email
    val g = lazy me'name
    users'filter(x => x'active)
.
"#;
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        let module = parser.parse_module().expect("module should parse");
        assert_eq!(module.items.len(), 1);
        match &module.items[0] {
            Item::Function(function) => {
                assert_eq!(function.body.len(), 3);
            }
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn parses_lazy_type_shorthand_in_params() {
        let src = r#"
fun debug(enabled: Bool, msg: lazy Str !io) -> Unit !io:
    print(msg())
.
"#;
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        let module = parser.parse_module().expect("module should parse");
        match &module.items[0] {
            Item::Function(function) => {
                assert_eq!(function.params.len(), 2);
                assert_eq!(function.effects, vec!["io"]);
                assert_eq!(
                    function.params[1].ty,
                    TypeExpr::Function {
                        params: vec![],
                        ret: Box::new(TypeExpr::Name("Str".to_string())),
                        effects: vec!["io".to_string()],
                    }
                );
            }
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn parses_match_with_constructor_and_wildcard_patterns() {
        let src = r#"
fun unwrap(x: Result) -> Int:
    match x:
        Ok(v):
            v
        Err(_):
            0
    .
.
"#;
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        let module = parser.parse_module().expect("module should parse");
        match &module.items[0] {
            Item::Function(function) => match &function.body[0] {
                Stmt::Expr(Expr::Match { arms, .. }) => {
                    assert_eq!(arms.len(), 2);
                    assert_eq!(
                        arms[0].pattern,
                        Pattern::Constructor {
                            name: "Ok".to_string(),
                            args: vec![Pattern::Name("v".to_string())],
                        }
                    );
                    assert_eq!(
                        arms[1].pattern,
                        Pattern::Constructor {
                            name: "Err".to_string(),
                            args: vec![Pattern::Wildcard],
                        }
                    );
                }
                other => panic!("expected match expression, got {other:?}"),
            },
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn parses_if_with_elif_and_else() {
        let src = r#"
fun classify(flag: Bool, other: Bool) -> Str:
    if flag:
        "yes"
    elif other:
        "maybe"
    else:
        "no"
    .
.
"#;
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        let module = parser.parse_module().expect("module should parse");
        match &module.items[0] {
            Item::Function(function) => match &function.body[0] {
                Stmt::Expr(Expr::If {
                    branches,
                    else_branch,
                }) => {
                    assert_eq!(branches.len(), 2);
                    assert!(else_branch.is_some());
                }
                other => panic!("expected if expression, got {other:?}"),
            },
            other => panic!("expected function, got {other:?}"),
        }
    }
}
