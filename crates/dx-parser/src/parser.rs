use crate::ast::{
    Arg, BinOp, Expr, FunctionDecl, ImportPyDecl, Item, LambdaBody, LambdaParam, MatchArm,
    Module, Param, Pattern, SchemaDecl, Stmt, TypeExpr,
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
        if self.at_keyword(Keyword::Schema) {
            return Ok(Item::Schema(self.parse_schema_decl()?));
        }
        if self.at_keyword(Keyword::Fun) {
            return Ok(Item::Function(self.parse_function_decl()?));
        }
        Ok(Item::Statement(self.parse_stmt()?))
    }

    fn parse_schema_decl(&mut self) -> Result<SchemaDecl, ParseError> {
        self.expect_keyword(Keyword::Schema)?;
        let name = self.expect_identifier()?;
        self.expect(TokenKind::Equal)?;
        let provider = self.expect_identifier()?;
        self.expect(TokenKind::Dot)?;
        self.expect_schema_member()?;
        self.expect(TokenKind::LParen)?;
        let source = self.expect_string()?;
        self.expect(TokenKind::RParen)?;

        let using_artifact = if self.at_identifier("using") {
            self.bump();
            Some(self.expect_string()?)
        } else {
            None
        };

        let refresh = if self.at_identifier("refresh") {
            self.bump();
            true
        } else {
            false
        };

        self.consume_optional_newline();
        Ok(SchemaDecl {
            name,
            provider,
            source,
            using_artifact,
            refresh,
        })
    }

    fn expect_schema_member(&mut self) -> Result<(), ParseError> {
        if self.at_keyword(Keyword::Schema) {
            self.bump();
            return Ok(());
        }

        let name = self.expect_identifier()?;
        if name == "schema" {
            return Ok(());
        }

        Err(ParseError::new("expected schema provider call"))
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
        self.expect_with_context(TokenKind::Colon, Some("after function signature"))?;
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
            self.expect_with_context(TokenKind::Colon, Some("after parameter name"))?;
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
        Ok(TypeExpr::Name(self.parse_named_type_text()?))
    }

    fn parse_named_type_text(&mut self) -> Result<String, ParseError> {
        let mut name = self.expect_identifier()?;

        while self.at(TokenKind::Dot) {
            self.bump();
            let segment = self.expect_identifier()?;
            name.push('.');
            name.push_str(&segment);
        }

        if self.at(TokenKind::LParen) {
            self.bump();
            let mut args = Vec::new();
            if !self.at(TokenKind::RParen) {
                loop {
                    args.push(self.parse_type_expr_text()?);
                    if !self.at(TokenKind::Comma) {
                        break;
                    }
                    self.bump();
                }
            }
            self.expect(TokenKind::RParen)?;
            name.push('(');
            name.push_str(&args.join(", "));
            name.push(')');
        }

        Ok(name)
    }

    fn parse_type_expr_text(&mut self) -> Result<String, ParseError> {
        match self.parse_type_expr()? {
            TypeExpr::Name(name) => Ok(name),
            TypeExpr::Function {
                params,
                ret,
                effects,
            } => {
                let mut out = String::from("(");
                let params: Vec<String> = params
                    .into_iter()
                    .map(|param| self.render_type_expr_text(param))
                    .collect();
                out.push_str(&params.join(", "));
                out.push_str(") -> ");
                out.push_str(&self.render_type_expr_text(*ret));
                for effect in effects {
                    out.push(' ');
                    out.push('!');
                    out.push_str(&effect);
                }
                Ok(out)
            }
        }
    }

    fn render_type_expr_text(&self, ty: TypeExpr) -> String {
        match ty {
            TypeExpr::Name(name) => name,
            TypeExpr::Function {
                params,
                ret,
                effects,
            } => {
                let mut out = String::from("(");
                let params: Vec<String> = params
                    .into_iter()
                    .map(|param| self.render_type_expr_text(param))
                    .collect();
                out.push_str(&params.join(", "));
                out.push_str(") -> ");
                out.push_str(&self.render_type_expr_text(*ret));
                for effect in effects {
                    out.push(' ');
                    out.push('!');
                    out.push_str(&effect);
                }
                out
            }
        }
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

        self.parse_comparison()
    }

    /// Precedence level: comparisons (`<`, `<=`, `>`, `>=`, `==`) — lowest binary precedence.
    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_additive()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::Lt) => BinOp::Lt,
                Some(TokenKind::LtEq) => BinOp::LtEq,
                Some(TokenKind::Gt) => BinOp::Gt,
                Some(TokenKind::GtEq) => BinOp::GtEq,
                Some(TokenKind::EqEq) => BinOp::EqEq,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_additive()?;
            lhs = Expr::BinaryOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    /// Precedence level: additive (`+`, `-`).
    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_multiplicative()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::Plus) => BinOp::Add,
                Some(TokenKind::Minus) => BinOp::Sub,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_multiplicative()?;
            lhs = Expr::BinaryOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    /// Precedence level: multiplicative (`*`).
    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_postfix_expr()?;
        loop {
            if !matches!(self.peek_kind(), Some(TokenKind::Star)) {
                break;
            }
            self.bump();
            let rhs = self.parse_postfix_expr()?;
            lhs = Expr::BinaryOp {
                op: BinOp::Mul,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_if_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect_keyword(Keyword::If)?;
        let cond = self.parse_expr()?;
        self.expect_with_context(TokenKind::Colon, Some("after `if` condition"))?;
        let then_body = self.parse_block_stmts()?;

        let mut branches = vec![(cond, then_body)];

        while self.at_keyword(Keyword::Elif) {
            self.bump();
            let elif_cond = self.parse_expr()?;
            self.expect_with_context(TokenKind::Colon, Some("after `elif` condition"))?;
            let elif_body = self.parse_block_stmts()?;
            branches.push((elif_cond, elif_body));
        }

        let else_branch = if self.at_keyword(Keyword::Else) {
            self.bump();
            self.expect_with_context(TokenKind::Colon, Some("after `else`"))?;
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
        self.expect_with_context(TokenKind::Colon, Some("after `match` scrutinee"))?;

        let mut arms = Vec::new();
        self.skip_newlines();
        while !self.at(TokenKind::Dot) {
            if self.at_eof() {
                return Err(ParseError::new("unterminated match expression"));
            }
            let pattern = self.parse_pattern()?;
            self.expect_with_context(TokenKind::Colon, Some("after match pattern"))?;
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
                if self.at(TokenKind::RParen) {
                    self.bump();
                    return Ok(Expr::Unit);
                }
                let expr = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            _ => Err(ParseError::new(format!(
                "expected expression, found {}",
                self.describe_current()
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
            _ => Err(ParseError::new(format!(
                "expected pattern (name, `_`, or constructor), found {}",
                self.describe_current()
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
            Err(ParseError::new(format!(
                "expected `{}`, found {}",
                keyword_str(&keyword),
                self.describe_current()
            )))
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<(), ParseError> {
        self.expect_with_context(kind, None)
    }

    fn expect_with_context(
        &mut self,
        kind: TokenKind,
        context: Option<&str>,
    ) -> Result<(), ParseError> {
        if self.at(kind.clone()) {
            self.bump();
            Ok(())
        } else {
            let expected = token_display(&kind);
            let msg = match context {
                Some(ctx) => format!("expected {} {}, found {}", expected, ctx, self.describe_current()),
                None => format!("expected {}, found {}", expected, self.describe_current()),
            };
            Err(ParseError::new(msg))
        }
    }

    fn expect_identifier(&mut self) -> Result<String, ParseError> {
        match self.peek_kind().cloned() {
            Some(TokenKind::Identifier(name)) => {
                self.bump();
                Ok(name)
            }
            _ => Err(ParseError::new(format!(
                "expected identifier, found {}",
                self.describe_current()
            ))),
        }
    }

    fn expect_string(&mut self) -> Result<String, ParseError> {
        match self.peek_kind().cloned() {
            Some(TokenKind::String(value)) => {
                self.bump();
                Ok(value)
            }
            _ => Err(ParseError::new(format!(
                "expected string literal, found {}",
                self.describe_current()
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
            _ => Err(ParseError::new(format!(
                "expected effect name after `!`, found {}",
                self.describe_current()
            ))),
        }
    }

    fn describe_current(&self) -> String {
        match self.peek_kind() {
            Some(TokenKind::Eof) | None => "end of input".to_string(),
            Some(TokenKind::Newline) => "newline".to_string(),
            Some(kind) => token_display(kind),
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

    fn at_identifier(&self, expected: &str) -> bool {
        matches!(self.peek_kind(), Some(TokenKind::Identifier(name)) if name == expected)
    }
}

fn token_display(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Colon => "`:`".to_string(),
        TokenKind::Dot => "`.`".to_string(),
        TokenKind::Apostrophe => "`'`".to_string(),
        TokenKind::Arrow => "`->`".to_string(),
        TokenKind::FatArrow => "`=>`".to_string(),
        TokenKind::Bang => "`!`".to_string(),
        TokenKind::Comma => "`,`".to_string(),
        TokenKind::LParen => "`(`".to_string(),
        TokenKind::RParen => "`)`".to_string(),
        TokenKind::Star => "`*`".to_string(),
        TokenKind::Plus => "`+`".to_string(),
        TokenKind::Minus => "`-`".to_string(),
        TokenKind::Lt => "`<`".to_string(),
        TokenKind::LtEq => "`<=`".to_string(),
        TokenKind::Gt => "`>`".to_string(),
        TokenKind::GtEq => "`>=`".to_string(),
        TokenKind::EqEq => "`==`".to_string(),
        TokenKind::Underscore => "`_`".to_string(),
        TokenKind::Equal => "`=`".to_string(),
        TokenKind::Ellipsis => "`...`".to_string(),
        TokenKind::Identifier(name) => format!("identifier `{name}`"),
        TokenKind::Integer(val) => format!("integer `{val}`"),
        TokenKind::String(val) => format!("string \"{}\"", val),
        TokenKind::Keyword(kw) => format!("`{}`", keyword_str(kw)),
        TokenKind::Newline => "newline".to_string(),
        TokenKind::Eof => "end of input".to_string(),
        TokenKind::Unknown(c) => format!("unexpected character `{c}`"),
    }
}

fn keyword_str(kw: &Keyword) -> &'static str {
    match kw {
        Keyword::Schema => "schema",
        Keyword::Fun => "fun",
        Keyword::If => "if",
        Keyword::Elif => "elif",
        Keyword::Else => "else",
        Keyword::Type => "type",
        Keyword::Lazy => "lazy",
        Keyword::Val => "val",
        Keyword::Var => "var",
        Keyword::Match => "match",
        Keyword::From => "from",
        Keyword::Import => "import",
        Keyword::Py => "py",
        Keyword::Me => "me",
        Keyword::It => "it",
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
    fn parses_schema_declaration() {
        let src = r#"
schema Customers = csv.schema("data/customers.csv") using "schemas/customers.dxschema" refresh
"#;
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        let module = parser.parse_module().expect("module should parse");
        assert_eq!(module.items.len(), 1);
        match &module.items[0] {
            Item::Schema(schema) => {
                assert_eq!(schema.name, "Customers");
                assert_eq!(schema.provider, "csv");
                assert_eq!(schema.source, "data/customers.csv");
                assert_eq!(
                    schema.using_artifact.as_deref(),
                    Some("schemas/customers.dxschema")
                );
                assert!(schema.refresh);
            }
            other => panic!("expected schema decl, got {other:?}"),
        }
    }

    // NOTE: lazy type shorthand in params is tested by lazy_param_type_with_io
    // NOTE: match with constructor/wildcard patterns is tested by match_nested_constructor_patterns

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

    // ── helpers ──────────────────────────────────────────────────

    fn parse(src: &str) -> Module {
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        parser.parse_module().expect("should parse")
    }

    fn parse_err(src: &str) -> String {
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        parser
            .parse_module()
            .expect_err("should fail to parse")
            .message
    }

    fn first_fun(module: &Module) -> &FunctionDecl {
        match &module.items[0] {
            Item::Function(f) => f,
            other => panic!("expected function, got {other:?}"),
        }
    }

    fn first_stmt_expr(stmts: &[Stmt]) -> &Expr {
        match &stmts[0] {
            Stmt::Expr(e) => e,
            other => panic!("expected expr stmt, got {other:?}"),
        }
    }

    // ── member access chains ─────────────────────────────────────

    #[test]
    fn deep_member_chain() {
        let m = parse(
            "fun f(u: User) -> Str:\n    u'account'primary_address'city\n.\n",
        );
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Member { base, name } => {
                assert_eq!(name, "city");
                match base.as_ref() {
                    Expr::Member { base, name } => {
                        assert_eq!(name, "primary_address");
                        match base.as_ref() {
                            Expr::Member { base, name } => {
                                assert_eq!(name, "account");
                                assert!(matches!(base.as_ref(), Expr::Name(n) if n == "u"));
                            }
                            other => panic!("expected Member, got {other:?}"),
                        }
                    }
                    other => panic!("expected Member, got {other:?}"),
                }
            }
            other => panic!("expected Member, got {other:?}"),
        }
    }

    #[test]
    fn six_level_member_chain() {
        let m = parse(
            "fun f(c: Config) -> Str:\n    c'environments'prod'services'api'base_url\n.\n",
        );
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Member { name, .. } => assert_eq!(name, "base_url"),
            other => panic!("expected Member, got {other:?}"),
        }
    }

    #[test]
    fn member_chain_with_call() {
        let m = parse("fun f(u: U) -> U:\n    u'filter(x => x'active)\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Call { callee, args } => {
                match callee.as_ref() {
                    Expr::Member { name, .. } => assert_eq!(name, "filter"),
                    other => panic!("expected Member callee, got {other:?}"),
                }
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn chained_member_calls() {
        let m = parse("fun f() -> Int:\n    me'first'len()\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Call { callee, args } => {
                assert!(args.is_empty());
                match callee.as_ref() {
                    Expr::Member { base, name } => {
                        assert_eq!(name, "len");
                        match base.as_ref() {
                            Expr::Member { name, .. } => assert_eq!(name, "first"),
                            other => panic!("expected Member, got {other:?}"),
                        }
                    }
                    other => panic!("expected Member, got {other:?}"),
                }
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    // ── placeholder with member access ───────────────────────────

    #[test]
    fn placeholder_member() {
        let m = parse("fun f() -> Int:\n    _'email\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Member { base, name } => {
                assert_eq!(name, "email");
                assert!(matches!(base.as_ref(), Expr::Placeholder));
            }
            other => panic!("expected Member, got {other:?}"),
        }
    }

    #[test]
    fn placeholder_in_call_arg() {
        let m = parse("fun f(u: U) -> I:\n    u'map(_'id)\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Call { args, .. } => {
                assert_eq!(args.len(), 1);
                match &args[0] {
                    Arg::Positional(Expr::Member { base, name }) => {
                        assert_eq!(name, "id");
                        assert!(matches!(base.as_ref(), Expr::Placeholder));
                    }
                    other => panic!("expected _'id, got {other:?}"),
                }
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    // ── lazy expressions ─────────────────────────────────────────

    #[test]
    fn lazy_simple_expr() {
        let m = parse("fun f() -> Unit:\n    lazy me'name\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Lazy { body: LambdaBody::Expr(inner) } => {
                assert!(matches!(inner.as_ref(), Expr::Member { name, .. } if name == "name"));
            }
            other => panic!("expected Lazy, got {other:?}"),
        }
    }

    #[test]
    fn lazy_block_form() {
        let m = parse("fun f() -> Unit:\n    lazy:\n        me'name\n    .\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Lazy { body: LambdaBody::Block(stmts) } => assert_eq!(stmts.len(), 1),
            other => panic!("expected Lazy block, got {other:?}"),
        }
    }

    #[test]
    fn lazy_wrapping_call() {
        let m = parse("fun f(p: Str) -> Unit:\n    lazy read_text(p)\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Lazy { body: LambdaBody::Expr(inner) } => {
                assert!(matches!(inner.as_ref(), Expr::Call { .. }));
            }
            other => panic!("expected Lazy(Call), got {other:?}"),
        }
    }

    #[test]
    fn lazy_param_type_no_effects() {
        let m = parse("fun f(fb: lazy T) -> T:\n    fb()\n.\n");
        let f = first_fun(&m);
        assert_eq!(
            f.params[0].ty,
            TypeExpr::Function {
                params: vec![],
                ret: Box::new(TypeExpr::Name("T".into())),
                effects: vec![],
            }
        );
    }

    #[test]
    fn lazy_param_type_with_io() {
        let m = parse("fun f(c: lazy Value !io) -> Value !io:\n    c()\n.\n");
        let f = first_fun(&m);
        assert_eq!(
            f.params[0].ty,
            TypeExpr::Function {
                params: vec![],
                ret: Box::new(TypeExpr::Name("Value".into())),
                effects: vec!["io".into()],
            }
        );
    }

    #[test]
    fn lazy_param_type_multiple_effects() {
        let m = parse("fun f(c: lazy Str !io !throw) -> Str:\n    c()\n.\n");
        let f = first_fun(&m);
        assert_eq!(
            f.params[0].ty,
            TypeExpr::Function {
                params: vec![],
                ret: Box::new(TypeExpr::Name("Str".into())),
                effects: vec!["io".into(), "throw".into()],
            }
        );
    }

    #[test]
    fn qualified_type_name_in_param_and_return() {
        let m = parse(
            "fun f(row: Customers.Row) -> Sales.Row:\n    row\n.\n",
        );
        let f = first_fun(&m);
        assert_eq!(f.params[0].ty, TypeExpr::Name("Customers.Row".into()));
        assert_eq!(f.return_type, Some(TypeExpr::Name("Sales.Row".into())));
    }

    #[test]
    fn applied_type_name_with_qualified_argument() {
        let m = parse(
            "fun f(rows: List(Customers.Row)) -> List(Sales.Row):\n    rows\n.\n",
        );
        let f = first_fun(&m);
        assert_eq!(
            f.params[0].ty,
            TypeExpr::Name("List(Customers.Row)".into())
        );
        assert_eq!(
            f.return_type,
            Some(TypeExpr::Name("List(Sales.Row)".into()))
        );
    }

    // ── named arguments ──────────────────────────────────────────

    #[test]
    fn named_args_only() {
        let m = parse("fun f() -> Unit:\n    connect(host: \"db\", port: 5432)\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Call { args, .. } => {
                assert_eq!(args.len(), 2);
                assert!(matches!(&args[0], Arg::Named { name, .. } if name == "host"));
                assert!(matches!(&args[1], Arg::Named { name, .. } if name == "port"));
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn mixed_positional_and_named_args() {
        let m = parse("fun f() -> Unit:\n    foo(1, key: 2)\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Call { args, .. } => {
                assert_eq!(args.len(), 2);
                assert!(matches!(&args[0], Arg::Positional(Expr::Integer(_))));
                assert!(matches!(&args[1], Arg::Named { name, .. } if name == "key"));
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    // ── lambda expressions ───────────────────────────────────────

    #[test]
    fn single_param_lambda() {
        let m = parse("fun f() -> Unit:\n    x => x\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Lambda { params, body } => {
                assert_eq!(params.len(), 1);
                assert_eq!(params[0].name, "x");
                assert!(params[0].ty.is_none());
                assert!(matches!(body, LambdaBody::Expr(_)));
            }
            other => panic!("expected Lambda, got {other:?}"),
        }
    }

    #[test]
    fn multi_param_lambda() {
        let m = parse("fun f() -> Unit:\n    (a, b) => a\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Lambda { params, .. } => {
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].name, "a");
                assert_eq!(params[1].name, "b");
            }
            other => panic!("expected Lambda, got {other:?}"),
        }
    }

    #[test]
    fn typed_lambda() {
        let m = parse("fun f() -> Unit:\n    (x: Int) => x\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Lambda { params, .. } => {
                assert_eq!(params[0].name, "x");
                assert_eq!(params[0].ty, Some(TypeExpr::Name("Int".into())));
            }
            other => panic!("expected Lambda, got {other:?}"),
        }
    }

    #[test]
    fn block_lambda() {
        let m = parse(
            "fun f() -> Unit:\n    x =>:\n        val y = x\n        y\n    .\n.\n",
        );
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Lambda { params, body: LambdaBody::Block(stmts) } => {
                assert_eq!(params[0].name, "x");
                assert_eq!(stmts.len(), 2);
            }
            other => panic!("expected block Lambda, got {other:?}"),
        }
    }

    #[test]
    fn lambda_as_call_arg() {
        let m = parse("fun f(u: U) -> U:\n    u'filter(x => x'active)\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Call { args, .. } => {
                assert_eq!(args.len(), 1);
                match &args[0] {
                    Arg::Positional(Expr::Lambda { params, .. }) => {
                        assert_eq!(params[0].name, "x");
                    }
                    other => panic!("expected lambda arg, got {other:?}"),
                }
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn multi_param_typed_lambda() {
        let m = parse("fun f() -> Unit:\n    (a: Int, b: Int) => a\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Lambda { params, .. } => {
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].ty, Some(TypeExpr::Name("Int".into())));
                assert_eq!(params[1].ty, Some(TypeExpr::Name("Int".into())));
            }
            other => panic!("expected Lambda, got {other:?}"),
        }
    }

    // ── if / elif / else ─────────────────────────────────────────

    #[test]
    fn simple_if_else() {
        let m = parse(
            "fun f(x: Bool) -> Str:\n    if x:\n        \"yes\"\n    else:\n        \"no\"\n    .\n.\n",
        );
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::If { branches, else_branch } => {
                assert_eq!(branches.len(), 1);
                assert!(matches!(&branches[0].0, Expr::Name(n) if n == "x"));
                assert!(else_branch.is_some());
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    #[test]
    fn if_without_else() {
        let m = parse("fun f(x: Bool) -> Unit:\n    if x:\n        print(\"hi\")\n    .\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::If { branches, else_branch } => {
                assert_eq!(branches.len(), 1);
                assert!(else_branch.is_none());
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    #[test]
    fn if_as_val_binding() {
        let m = parse(
            "fun f(x: Bool) -> Str:\n    val r = if x:\n        \"y\"\n    else:\n        \"n\"\n    .\n    r\n.\n",
        );
        let f = first_fun(&m);
        assert_eq!(f.body.len(), 2);
        match &f.body[0] {
            Stmt::ValBind { name, value } => {
                assert_eq!(name, "r");
                assert!(matches!(value, Expr::If { .. }));
            }
            other => panic!("expected ValBind(If), got {other:?}"),
        }
    }

    #[test]
    fn nested_if() {
        let m = parse(
            "fun f(a: Bool, b: Bool) -> Str:\n    if a:\n        if b:\n            \"ab\"\n        else:\n            \"a\"\n        .\n    else:\n        \"none\"\n    .\n.\n",
        );
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::If { branches, else_branch } => {
                assert_eq!(branches.len(), 1);
                match first_stmt_expr(&branches[0].1) {
                    Expr::If { branches: inner, .. } => assert_eq!(inner.len(), 1),
                    other => panic!("expected nested If, got {other:?}"),
                }
                assert!(else_branch.is_some());
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    #[test]
    fn if_with_multiple_body_stmts() {
        let m = parse(
            "fun f(x: Bool) -> Unit:\n    if x:\n        val a = 1\n        val b = 2\n        print(a)\n    .\n.\n",
        );
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::If { branches, .. } => {
                assert_eq!(branches[0].1.len(), 3);
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    // ── match expressions ────────────────────────────────────────

    #[test]
    fn match_simple_name_patterns() {
        let m = parse(
            "fun f(x: T) -> Int:\n    match x:\n        a:\n            1\n        b:\n            2\n    .\n.\n",
        );
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Match { arms, .. } => {
                assert_eq!(arms.len(), 2);
                assert_eq!(arms[0].pattern, Pattern::Name("a".into()));
                assert_eq!(arms[1].pattern, Pattern::Name("b".into()));
            }
            other => panic!("expected Match, got {other:?}"),
        }
    }

    #[test]
    fn match_with_wildcard() {
        let m = parse(
            "fun f(x: T) -> Int:\n    match x:\n        a:\n            1\n        _:\n            0\n    .\n.\n",
        );
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Match { arms, .. } => {
                assert_eq!(arms[1].pattern, Pattern::Wildcard);
            }
            other => panic!("expected Match, got {other:?}"),
        }
    }

    #[test]
    fn match_nested_constructor_patterns() {
        let m = parse(
            "fun f(x: T) -> Int:\n    match x:\n        Some(Ok(v)):\n            v\n        _:\n            0\n    .\n.\n",
        );
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Match { arms, .. } => {
                match &arms[0].pattern {
                    Pattern::Constructor { name, args } => {
                        assert_eq!(name, "Some");
                        assert_eq!(args.len(), 1);
                        match &args[0] {
                            Pattern::Constructor { name, args } => {
                                assert_eq!(name, "Ok");
                                assert_eq!(args.len(), 1);
                            }
                            other => panic!("expected inner Constructor, got {other:?}"),
                        }
                    }
                    other => panic!("expected Constructor, got {other:?}"),
                }
            }
            other => panic!("expected Match, got {other:?}"),
        }
    }

    #[test]
    fn match_as_val_binding() {
        let m = parse(
            "fun f(x: T) -> Int:\n    val r = match x:\n        a:\n            1\n        _:\n            0\n    .\n    r\n.\n",
        );
        let f = first_fun(&m);
        match &f.body[0] {
            Stmt::ValBind { name, value } => {
                assert_eq!(name, "r");
                assert!(matches!(value, Expr::Match { .. }));
            }
            other => panic!("expected ValBind(Match), got {other:?}"),
        }
    }

    // ── from py import ───────────────────────────────────────────

    #[test]
    fn from_py_import_single() {
        let m = parse("from py pandas import read_csv\n");
        match &m.items[0] {
            Item::ImportPy(imp) => {
                assert_eq!(imp.module, "pandas");
                assert_eq!(imp.names, vec!["read_csv"]);
            }
            other => panic!("expected ImportPy, got {other:?}"),
        }
    }

    #[test]
    fn from_py_import_multi() {
        let m = parse("from py builtins import print, len, range\n");
        match &m.items[0] {
            Item::ImportPy(imp) => {
                assert_eq!(imp.names, vec!["print", "len", "range"]);
            }
            other => panic!("expected ImportPy, got {other:?}"),
        }
    }

    #[test]
    fn import_then_function() {
        let m = parse(
            "from py numpy import array\n\nfun f(x: Xs) -> PyObj !py !throw:\n    array(x)\n.\n",
        );
        assert_eq!(m.items.len(), 2);
        assert!(matches!(&m.items[0], Item::ImportPy(_)));
        assert!(matches!(&m.items[1], Item::Function(_)));
    }

    // ── function types in annotations ────────────────────────────

    #[test]
    fn function_type_return_annotation() {
        let m = parse("fun f() -> (User, User) -> Bool:\n    me\n.\n");
        let f = first_fun(&m);
        match &f.return_type {
            Some(TypeExpr::Function { params, ret, effects }) => {
                assert_eq!(params.len(), 2);
                assert_eq!(**ret, TypeExpr::Name("Bool".into()));
                assert!(effects.is_empty());
            }
            other => panic!("expected Function type, got {other:?}"),
        }
    }

    #[test]
    fn function_type_with_effects() {
        let m = parse("fun f() -> () -> T !io:\n    me\n.\n");
        let f = first_fun(&m);
        match &f.return_type {
            Some(TypeExpr::Function { params, effects, .. }) => {
                assert!(params.is_empty());
                assert_eq!(effects, &["io"]);
            }
            other => panic!("expected Function type, got {other:?}"),
        }
    }

    // ── val / var / rebind ───────────────────────────────────────

    #[test]
    fn val_binding() {
        let m = parse("fun f() -> Unit:\n    val x = 42\n    x\n.\n");
        let f = first_fun(&m);
        match &f.body[0] {
            Stmt::ValBind { name, value } => {
                assert_eq!(name, "x");
                assert!(matches!(value, Expr::Integer(s) if s == "42"));
            }
            other => panic!("expected ValBind, got {other:?}"),
        }
    }

    #[test]
    fn var_and_rebind() {
        let m = parse("fun f() -> Unit:\n    var x = 1\n    x = 2\n.\n");
        let f = first_fun(&m);
        assert!(matches!(&f.body[0], Stmt::VarBind { name, .. } if name == "x"));
        assert!(matches!(&f.body[1], Stmt::Rebind { name, .. } if name == "x"));
    }

    // ── effects on functions ─────────────────────────────────────

    #[test]
    fn multiple_effects() {
        let m = parse("fun f(p: Str) -> PyObj !py !throw !io:\n    p\n.\n");
        let f = first_fun(&m);
        assert_eq!(f.effects, vec!["py", "throw", "io"]);
    }

    #[test]
    fn no_effects() {
        let m = parse("fun f() -> Int:\n    42\n.\n");
        let f = first_fun(&m);
        assert!(f.effects.is_empty());
    }

    #[test]
    fn no_return_type() {
        let m = parse("fun f():\n    42\n.\n");
        let f = first_fun(&m);
        assert!(f.return_type.is_none());
    }

    // ── me and it ────────────────────────────────────────────────

    #[test]
    fn me_member_access() {
        let m = parse("fun f() -> Str:\n    me'first\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Member { base, name } => {
                assert_eq!(name, "first");
                assert!(matches!(base.as_ref(), Expr::Name(n) if n == "me"));
            }
            other => panic!("expected Member, got {other:?}"),
        }
    }

    #[test]
    fn it_member_access_with_call() {
        let m = parse("fun f() -> Unit:\n    it'filter(x => x)\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Call { callee, .. } => match callee.as_ref() {
                Expr::Member { base, name } => {
                    assert_eq!(name, "filter");
                    assert!(matches!(base.as_ref(), Expr::Name(n) if n == "it"));
                }
                other => panic!("expected Member, got {other:?}"),
            },
            other => panic!("expected Call, got {other:?}"),
        }
    }

    // ── multi-item modules ───────────────────────────────────────

    #[test]
    fn multiple_functions() {
        let m = parse(
            "fun f() -> Int:\n    1\n.\n\nfun g() -> Int:\n    2\n.\n",
        );
        assert_eq!(m.items.len(), 2);
        assert!(matches!(&m.items[0], Item::Function(f) if f.name == "f"));
        assert!(matches!(&m.items[1], Item::Function(f) if f.name == "g"));
    }

    #[test]
    fn import_and_multiple_functions() {
        let m = parse(
            "from py pandas import read_csv\n\nfun load(p: Str) -> PyObj !py !throw:\n    read_csv(p)\n.\n\nfun wrap(p: Str) -> PyObj !py !throw:\n    load(p)\n.\n",
        );
        assert_eq!(m.items.len(), 3);
    }

    // ── error cases: blocks ─────────────────────────────────────

    #[test]
    fn error_unterminated_fun_block() {
        let msg = parse_err("fun f():\n    x\n");
        assert!(msg.contains("unterminated"), "got: {msg}");
    }

    #[test]
    fn error_unterminated_fun_block_with_operator() {
        let msg = parse_err("fun f(a: Int, b: Int) -> Int:\n    a + b\n");
        assert!(msg.contains("unterminated"), "got: {msg}");
    }

    #[test]
    fn error_missing_colon_after_fun_signature() {
        let msg = parse_err("fun f()\n    x\n.\n");
        assert!(!msg.is_empty());
    }

    // ── error cases: function declarations ───────────────────────

    #[test]
    fn error_missing_rparen_in_params() {
        let msg = parse_err("fun f(x: Int:\n    x\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_missing_param_type() {
        let msg = parse_err("fun f(x):\n    x\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_missing_fun_name() {
        let msg = parse_err("fun ():\n    1\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_missing_lparen() {
        let msg = parse_err("fun f:\n    1\n.\n");
        assert!(!msg.is_empty());
    }

    // ── error cases: if / elif / else ────────────────────────────

    #[test]
    fn error_if_missing_colon() {
        let msg = parse_err("fun f(x: Bool) -> Int:\n    if x\n        1\n    .\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_if_missing_dot() {
        let msg = parse_err("fun f(x: Bool) -> Int:\n    if x:\n        1\n.\n");
        // The outer fun block dot gets consumed by the if, leaving fun unterminated
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_elif_without_if() {
        // elif appearing at expression position should fail
        let msg = parse_err("fun f() -> Int:\n    elif x:\n        1\n    .\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_else_without_if() {
        let msg = parse_err("fun f() -> Int:\n    else:\n        1\n    .\n.\n");
        assert!(!msg.is_empty());
    }

    // ── error cases: match ───────────────────────────────────────

    #[test]
    fn error_match_missing_colon() {
        let msg = parse_err("fun f(x: T) -> Int:\n    match x\n        a:\n            1\n    .\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_match_missing_scrutinee() {
        let msg = parse_err("fun f() -> Int:\n    match:\n        a:\n            1\n    .\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_match_arm_missing_colon() {
        let msg = parse_err("fun f(x: T) -> Int:\n    match x:\n        a\n            1\n    .\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_match_unterminated() {
        let _msg = parse_err("fun f(x: T) -> Int:\n    match x:\n        a:\n            1\n.\n");
        // Just verify no panic
    }

    // ── error cases: operators ────────────────────────────────────

    #[test]
    fn error_trailing_operator() {
        let msg = parse_err("fun f(a: Int) -> Int:\n    a +\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_leading_operator() {
        let msg = parse_err("fun f(a: Int) -> Int:\n    * a\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_double_operator() {
        let msg = parse_err("fun f(a: Int, b: Int) -> Int:\n    a + + b\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_comparison_missing_rhs() {
        let msg = parse_err("fun f(x: Int) -> Bool:\n    x <\n.\n");
        assert!(!msg.is_empty());
    }

    // ── error cases: lambda ──────────────────────────────────────

    #[test]
    fn error_fat_arrow_without_params() {
        let msg = parse_err("fun f() -> Unit:\n    => x\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_block_lambda_unterminated() {
        let msg = parse_err("fun f() -> Unit:\n    x =>:\n        y\n.\n");
        // The outer fun's `.` gets consumed by the block lambda, leaving fun unterminated
        assert!(!msg.is_empty());
    }

    // ── error cases: lazy ────────────────────────────────────────

    #[test]
    fn error_lazy_at_eof() {
        let msg = parse_err("fun f() -> Unit:\n    lazy\n.\n");
        // lazy followed by newline then `.` — lazy tries to parse an expression,
        // sees `.` which is not a valid expression start
        assert!(!msg.is_empty());
    }

    // ── error cases: val / var ───────────────────────────────────

    #[test]
    fn error_val_missing_equals() {
        let msg = parse_err("fun f() -> Unit:\n    val x 1\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_val_missing_name() {
        let msg = parse_err("fun f() -> Unit:\n    val = 1\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_var_missing_equals() {
        let msg = parse_err("fun f() -> Unit:\n    var x 1\n.\n");
        assert!(!msg.is_empty());
    }

    // ── error cases: imports ─────────────────────────────────────

    #[test]
    fn error_import_missing_py() {
        let msg = parse_err("from pandas import read_csv\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_import_missing_module() {
        let msg = parse_err("from py import read_csv\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_import_missing_import_keyword() {
        let msg = parse_err("from py pandas read_csv\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_import_missing_names() {
        let msg = parse_err("from py pandas import\n");
        assert!(!msg.is_empty());
    }

    // ── error cases: expressions ─────────────────────────────────

    #[test]
    fn error_unclosed_paren() {
        let msg = parse_err("fun f() -> Int:\n    (1 + 2\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_unexpected_token_in_expression() {
        let msg = parse_err("fun f() -> Int:\n    :\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_empty_call_args_with_comma() {
        let msg = parse_err("fun f() -> Unit:\n    g(,)\n.\n");
        assert!(!msg.is_empty());
    }

    // ── error cases: constructor patterns ────────────────────────

    #[test]
    fn error_constructor_pattern_unclosed_paren() {
        let msg = parse_err("fun f(x: T) -> Int:\n    match x:\n        Ok(v:\n            v\n    .\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_constructor_pattern_nested_unclosed() {
        let msg = parse_err("fun f(x: T) -> Int:\n    match x:\n        Some(Ok(v):\n            v\n    .\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_integer_as_pattern() {
        let msg = parse_err("fun f(x: T) -> Int:\n    match x:\n        42:\n            1\n    .\n.\n");
        assert!(!msg.is_empty());
    }

    // ── error cases: lazy param type syntax ──────────────────────

    #[test]
    fn error_lazy_type_missing_type_name() {
        // lazy with nothing after it in type position — hits rparen or comma
        let msg = parse_err("fun f(x: lazy) -> Unit:\n    x()\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_lazy_type_bang_without_effect_name() {
        let msg = parse_err("fun f(x: lazy T !) -> Unit:\n    x()\n.\n");
        assert!(!msg.is_empty());
    }

    // ── error cases: operators in complex contexts ───────────────

    #[test]
    fn error_trailing_op_in_if_condition() {
        let msg = parse_err("fun f(a: Int) -> Int:\n    if a +:\n        1\n    .\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_trailing_op_in_match_scrutinee() {
        let msg = parse_err("fun f(a: Int) -> Int:\n    match a +:\n        x:\n            1\n    .\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_trailing_op_in_named_arg() {
        let msg = parse_err("fun f() -> Unit:\n    foo(key: a +)\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_trailing_op_in_val() {
        let msg = parse_err("fun f() -> Unit:\n    val x = a *\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_trailing_op_in_lambda_body() {
        let msg = parse_err("fun f() -> Unit:\n    x => x +\n.\n");
        assert!(!msg.is_empty());
    }

    // ── error cases: paren mismatches ────────────────────────────

    #[test]
    fn error_unclosed_nested_parens() {
        let msg = parse_err("fun f() -> Int:\n    ((1 + 2)\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_unclosed_paren_in_call() {
        let msg = parse_err("fun f(x: X) -> Y:\n    g((x + 1)\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_extra_rparen() {
        let msg = parse_err("fun f() -> Int:\n    (1 + 2))\n.\n");
        assert!(!msg.is_empty());
    }

    // ── composite: if + match nesting ────────────────────────────

    #[test]
    fn match_inside_if_branch() {
        let m = parse(
            "fun f(flag: Bool, x: T) -> Int:\n    if flag:\n        match x:\n            a:\n                1\n            _:\n                0\n        .\n    else:\n        99\n    .\n.\n",
        );
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::If { branches, else_branch } => {
                // then-branch contains a match
                match first_stmt_expr(&branches[0].1) {
                    Expr::Match { arms, .. } => assert_eq!(arms.len(), 2),
                    other => panic!("expected Match in if-then, got {other:?}"),
                }
                assert!(else_branch.is_some());
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    #[test]
    fn if_inside_match_arm() {
        let m = parse(
            "fun f(x: T, flag: Bool) -> Str:\n    match x:\n        a:\n            if flag:\n                \"yes\"\n            else:\n                \"no\"\n            .\n        _:\n            \"default\"\n    .\n.\n",
        );
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Match { arms, .. } => {
                assert_eq!(arms.len(), 2);
                match first_stmt_expr(&arms[0].body) {
                    Expr::If { branches, .. } => assert_eq!(branches.len(), 1),
                    other => panic!("expected If in match arm, got {other:?}"),
                }
            }
            other => panic!("expected Match, got {other:?}"),
        }
    }

    // ── composite: lazy in various positions ─────────────────────

    #[test]
    fn lazy_as_second_call_arg() {
        let m = parse("fun f(u: User) -> Str:\n    get(u'nick, lazy u'full_name())\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Call { args, .. } => {
                assert_eq!(args.len(), 2);
                match &args[1] {
                    Arg::Positional(Expr::Lazy { body: LambdaBody::Expr(inner) }) => {
                        assert!(matches!(inner.as_ref(), Expr::Call { .. }));
                    }
                    other => panic!("expected lazy as 2nd arg, got {other:?}"),
                }
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn lazy_block_as_call_arg() {
        let m = parse("fun f(id: Int) -> Value !io:\n    compute(\"key\", lazy:\n        read(id)\n    .)\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Call { args, .. } => {
                assert_eq!(args.len(), 2);
                match &args[1] {
                    Arg::Positional(Expr::Lazy { body: LambdaBody::Block(stmts) }) => {
                        assert_eq!(stmts.len(), 1);
                    }
                    other => panic!("expected lazy block as arg, got {other:?}"),
                }
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn multiple_lazy_args() {
        let m = parse("fun f(a: A, b: B) -> Unit:\n    run(lazy a'x(), lazy b'y())\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Call { args, .. } => {
                assert_eq!(args.len(), 2);
                assert!(matches!(&args[0], Arg::Positional(Expr::Lazy { .. })));
                assert!(matches!(&args[1], Arg::Positional(Expr::Lazy { .. })));
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    // ── composite: member chain + call interleaving ──────────────

    #[test]
    fn member_call_member_call() {
        // db'conn'pool'acquire()'execute("q")
        let m = parse("fun f(db: Db) -> R:\n    db'conn'pool'acquire()'execute(\"q\")\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Call { callee, args } => {
                // outermost is 'execute("q")
                assert_eq!(args.len(), 1);
                match callee.as_ref() {
                    Expr::Member { name, .. } => assert_eq!(name, "execute"),
                    other => panic!("expected Member, got {other:?}"),
                }
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn lambda_body_with_member_chain() {
        let m = parse("fun f(u: U) -> U:\n    u'map(x => x'name'len())\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Call { args, .. } => match &args[0] {
                Arg::Positional(Expr::Lambda { body: LambdaBody::Expr(inner), .. }) => {
                    // x'name'len() -> Call(Member(Member(Name, name), len), [])
                    assert!(matches!(inner.as_ref(), Expr::Call { .. }));
                }
                other => panic!("expected lambda, got {other:?}"),
            },
            other => panic!("expected Call, got {other:?}"),
        }
    }

    // ── composite: match with multi-statement arms ───────────────

    #[test]
    fn match_arm_with_val_bindings() {
        let m = parse(
            "fun f(x: T) -> Int:\n    match x:\n        Ok(v):\n            val y = v\n            transform(y)\n        Err(_):\n            val z = default()\n            z\n    .\n.\n",
        );
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Match { arms, .. } => {
                assert_eq!(arms[0].body.len(), 2);
                assert!(matches!(&arms[0].body[0], Stmt::ValBind { name, .. } if name == "y"));
                assert_eq!(arms[1].body.len(), 2);
                assert!(matches!(&arms[1].body[0], Stmt::ValBind { name, .. } if name == "z"));
            }
            other => panic!("expected Match, got {other:?}"),
        }
    }

    // ── it pipeline ──────────────────────────────────────────────

    #[test]
    fn it_pipeline_two_steps() {
        let m = parse("fun f() -> Unit:\n    read(path)\n    it'filter(x => x'active)\n    it'map(x => x'email)\n.\n");
        let f = first_fun(&m);
        assert_eq!(f.body.len(), 3);
        // Second stmt: it'filter(...)
        match &f.body[1] {
            Stmt::Expr(Expr::Call { callee, .. }) => match callee.as_ref() {
                Expr::Member { base, name } => {
                    assert_eq!(name, "filter");
                    assert!(matches!(base.as_ref(), Expr::Name(n) if n == "it"));
                }
                other => panic!("expected it'filter, got {other:?}"),
            },
            other => panic!("expected Call, got {other:?}"),
        }
    }

    // ── deeply nested calls ──────────────────────────────────────

    #[test]
    fn four_deep_nested_calls() {
        let m = parse("fun f(x: X) -> Y:\n    a(b(c(d(x))))\n.\n");
        let f = first_fun(&m);
        // a(b(c(d(x)))) -> Call(a, [Call(b, [Call(c, [Call(d, [x])])])])
        match first_stmt_expr(&f.body) {
            Expr::Call { callee, args } => {
                assert!(matches!(callee.as_ref(), Expr::Name(n) if n == "a"));
                assert_eq!(args.len(), 1);
                match &args[0] {
                    Arg::Positional(Expr::Call { callee, .. }) => {
                        assert!(matches!(callee.as_ref(), Expr::Name(n) if n == "b"));
                    }
                    other => panic!("expected nested Call, got {other:?}"),
                }
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    // ── if/match used as val initializer then more stmts ─────────

    #[test]
    fn match_val_then_transform() {
        let m = parse(
            "fun f(x: T) -> Int:\n    val base = match x:\n        a:\n            1\n        _:\n            0\n    .\n    transform(base)\n.\n",
        );
        let f = first_fun(&m);
        assert_eq!(f.body.len(), 2);
        assert!(matches!(&f.body[0], Stmt::ValBind { value: Expr::Match { .. }, .. }));
        assert!(matches!(&f.body[1], Stmt::Expr(Expr::Call { .. })));
    }

    #[test]
    fn if_else_both_multi_stmt() {
        let m = parse(
            "fun f(x: Bool) -> Int:\n    if x:\n        val a = 1\n        val b = 2\n        combine(a, b)\n    else:\n        val c = 3\n        c\n    .\n.\n",
        );
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::If { branches, else_branch } => {
                assert_eq!(branches[0].1.len(), 3);
                assert_eq!(else_branch.as_ref().unwrap().len(), 2);
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    // ── lazy param with !py effect ───────────────────────────────

    #[test]
    fn lazy_param_with_py_effect() {
        let m = parse("fun f(action: lazy PyObj !py !throw) -> PyObj !py !throw:\n    action()\n.\n");
        let f = first_fun(&m);
        assert_eq!(
            f.params[0].ty,
            TypeExpr::Function {
                params: vec![],
                ret: Box::new(TypeExpr::Name("PyObj".into())),
                effects: vec!["py".into(), "throw".into()],
            }
        );
        assert_eq!(f.effects, vec!["py", "throw"]);
    }

    // ── regression: named args + lazy args in same call ──────────

    #[test]
    fn named_arg_with_lazy_value() {
        let m = parse("fun f(p: Str) -> Unit !io:\n    run(target: \"prod\", fallback: lazy load(p))\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Call { args, .. } => {
                assert_eq!(args.len(), 2);
                match &args[0] {
                    Arg::Named { name, value } => {
                        assert_eq!(name, "target");
                        assert!(matches!(value, Expr::String(s) if s == "prod"));
                    }
                    other => panic!("expected Named arg, got {other:?}"),
                }
                match &args[1] {
                    Arg::Named { name, value } => {
                        assert_eq!(name, "fallback");
                        assert!(matches!(value, Expr::Lazy { .. }));
                    }
                    other => panic!("expected Named(lazy), got {other:?}"),
                }
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn positional_and_named_lazy_mixed() {
        let m = parse("fun f(e: Bool, p: Str) -> Unit !io:\n    debug(e, msg: lazy read_text(p))\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Call { args, .. } => {
                assert_eq!(args.len(), 2);
                assert!(matches!(&args[0], Arg::Positional(Expr::Name(n)) if n == "e"));
                match &args[1] {
                    Arg::Named { name, value } => {
                        assert_eq!(name, "msg");
                        match value {
                            Expr::Lazy { body: LambdaBody::Expr(inner) } => {
                                assert!(matches!(inner.as_ref(), Expr::Call { .. }));
                            }
                            other => panic!("expected Lazy(Call), got {other:?}"),
                        }
                    }
                    other => panic!("expected Named arg, got {other:?}"),
                }
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    // ── regression: multiple from-py imports ─────────────────────

    #[test]
    fn multiple_py_imports_in_module() {
        let m = parse("from py pandas import read_csv, DataFrame\nfrom py numpy import array\n\nfun f(p: Str) -> PyObj !py !throw:\n    read_csv(p)\n.\n");
        assert_eq!(m.items.len(), 3);
        match &m.items[0] {
            Item::ImportPy(imp) => {
                assert_eq!(imp.module, "pandas");
                assert_eq!(imp.names, vec!["read_csv", "DataFrame"]);
            }
            other => panic!("expected ImportPy, got {other:?}"),
        }
        match &m.items[1] {
            Item::ImportPy(imp) => {
                assert_eq!(imp.module, "numpy");
                assert_eq!(imp.names, vec!["array"]);
            }
            other => panic!("expected ImportPy, got {other:?}"),
        }
        assert!(matches!(&m.items[2], Item::Function(_)));
    }

    // ── regression: match on member-access scrutinee ─────────────

    #[test]
    fn match_on_member_chain() {
        let m = parse("fun f(r: Resp) -> Int:\n    match r'status'code:\n        ok:\n            1\n        _:\n            0\n    .\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Match { scrutinee, arms } => {
                match scrutinee.as_ref() {
                    Expr::Member { base, name } => {
                        assert_eq!(name, "code");
                        assert!(matches!(base.as_ref(), Expr::Member { name, .. } if name == "status"));
                    }
                    other => panic!("expected Member chain scrutinee, got {other:?}"),
                }
                assert_eq!(arms.len(), 2);
            }
            other => panic!("expected Match, got {other:?}"),
        }
    }

    // ── regression: if with match in both branches ───────────────

    #[test]
    fn if_with_match_in_both_branches() {
        let m = parse(
            "fun f(flag: Bool, x: T, y: T) -> Int:\n    if flag:\n        match x:\n            a:\n                1\n            _:\n                0\n        .\n    else:\n        match y:\n            b:\n                2\n            _:\n                0\n        .\n    .\n.\n",
        );
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::If { branches, else_branch } => {
                assert!(matches!(first_stmt_expr(&branches[0].1), Expr::Match { .. }));
                assert!(matches!(
                    first_stmt_expr(else_branch.as_ref().unwrap()),
                    Expr::Match { .. }
                ));
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    // ── regression: multiple lazy params in one function ──────────

    #[test]
    fn multiple_lazy_params() {
        let m = parse("fun f(key: Str, compute: lazy Value !io, fallback: lazy Value) -> Value !io:\n    compute()\n.\n");
        let f = first_fun(&m);
        assert_eq!(f.params.len(), 3);
        assert_eq!(f.params[0].ty, TypeExpr::Name("Str".into()));
        assert_eq!(
            f.params[1].ty,
            TypeExpr::Function {
                params: vec![],
                ret: Box::new(TypeExpr::Name("Value".into())),
                effects: vec!["io".into()],
            }
        );
        assert_eq!(
            f.params[2].ty,
            TypeExpr::Function {
                params: vec![],
                ret: Box::new(TypeExpr::Name("Value".into())),
                effects: vec![],
            }
        );
    }

    // ── binary operators: basic ──────────────────────────────────

    #[test]
    fn simple_addition() {
        let m = parse("fun f(a: Int, b: Int) -> Int:\n    a + b\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::BinaryOp { op, lhs, rhs } => {
                assert_eq!(*op, BinOp::Add);
                assert!(matches!(lhs.as_ref(), Expr::Name(n) if n == "a"));
                assert!(matches!(rhs.as_ref(), Expr::Name(n) if n == "b"));
            }
            other => panic!("expected BinaryOp(Add), got {other:?}"),
        }
    }

    #[test]
    fn simple_subtraction() {
        let m = parse("fun f(n: Int) -> Int:\n    n - 1\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::Sub),
            other => panic!("expected BinaryOp(Sub), got {other:?}"),
        }
    }

    #[test]
    fn simple_multiplication() {
        let m = parse("fun f(a: Int, b: Int) -> Int:\n    a * b\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::Mul),
            other => panic!("expected BinaryOp(Mul), got {other:?}"),
        }
    }

    #[test]
    fn simple_lt() {
        let m = parse("fun f(x: Int) -> Bool:\n    x < 0\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::Lt),
            other => panic!("expected BinaryOp(Lt), got {other:?}"),
        }
    }

    #[test]
    fn simple_lt_eq() {
        let m = parse("fun f(n: Int) -> Bool:\n    n <= 1\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::LtEq),
            other => panic!("expected BinaryOp(LtEq), got {other:?}"),
        }
    }

    #[test]
    fn simple_eq_eq() {
        let m = parse("fun f(x: Int) -> Bool:\n    x == 0\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::EqEq),
            other => panic!("expected BinaryOp(EqEq), got {other:?}"),
        }
    }

    // ── binary operators: precedence ─────────────────────────────

    #[test]
    fn mul_binds_tighter_than_add() {
        // a + b * c  should parse as  a + (b * c)
        let m = parse("fun f(a: Int, b: Int, c: Int) -> Int:\n    a + b * c\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::BinaryOp { op, lhs, rhs } => {
                assert_eq!(*op, BinOp::Add);
                assert!(matches!(lhs.as_ref(), Expr::Name(n) if n == "a"));
                match rhs.as_ref() {
                    Expr::BinaryOp { op, lhs, rhs } => {
                        assert_eq!(*op, BinOp::Mul);
                        assert!(matches!(lhs.as_ref(), Expr::Name(n) if n == "b"));
                        assert!(matches!(rhs.as_ref(), Expr::Name(n) if n == "c"));
                    }
                    other => panic!("expected Mul on rhs, got {other:?}"),
                }
            }
            other => panic!("expected BinaryOp(Add), got {other:?}"),
        }
    }

    #[test]
    fn comparison_lower_than_arithmetic() {
        // a + b < c  should parse as  (a + b) < c
        let m = parse("fun f(a: Int, b: Int, c: Int) -> Bool:\n    a + b < c\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::BinaryOp { op, lhs, rhs } => {
                assert_eq!(*op, BinOp::Lt);
                match lhs.as_ref() {
                    Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::Add),
                    other => panic!("expected Add on lhs, got {other:?}"),
                }
                assert!(matches!(rhs.as_ref(), Expr::Name(n) if n == "c"));
            }
            other => panic!("expected BinaryOp(Lt), got {other:?}"),
        }
    }

    #[test]
    fn left_associative_addition() {
        // a + b + c  should parse as  (a + b) + c
        let m = parse("fun f(a: Int, b: Int, c: Int) -> Int:\n    a + b + c\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::BinaryOp { op, lhs, rhs } => {
                assert_eq!(*op, BinOp::Add);
                match lhs.as_ref() {
                    Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::Add),
                    other => panic!("expected Add on lhs, got {other:?}"),
                }
                assert!(matches!(rhs.as_ref(), Expr::Name(n) if n == "c"));
            }
            other => panic!("expected BinaryOp(Add), got {other:?}"),
        }
    }

    // ── binary operators: with member access and calls ───────────

    #[test]
    fn member_access_binds_tighter_than_operators() {
        // a'x + b'y  should parse as  (a'x) + (b'y)
        let m = parse("fun f(a: A, b: B) -> Int:\n    a'x + b'y\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::BinaryOp { op, lhs, rhs } => {
                assert_eq!(*op, BinOp::Add);
                assert!(matches!(lhs.as_ref(), Expr::Member { name, .. } if name == "x"));
                assert!(matches!(rhs.as_ref(), Expr::Member { name, .. } if name == "y"));
            }
            other => panic!("expected BinaryOp(Add), got {other:?}"),
        }
    }

    #[test]
    fn comparison_with_member_access() {
        // a'age < b'age
        let m = parse("fun f(a: User, b: User) -> Bool:\n    a'age < b'age\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::BinaryOp { op, lhs, rhs } => {
                assert_eq!(*op, BinOp::Lt);
                assert!(matches!(lhs.as_ref(), Expr::Member { name, .. } if name == "age"));
                assert!(matches!(rhs.as_ref(), Expr::Member { name, .. } if name == "age"));
            }
            other => panic!("expected BinaryOp(Lt), got {other:?}"),
        }
    }

    #[test]
    fn subtraction_in_call_arg() {
        // fact(n - 1)
        let m = parse("fun f(n: Int) -> Int:\n    f(n - 1)\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::Call { args, .. } => match &args[0] {
                Arg::Positional(Expr::BinaryOp { op, .. }) => assert_eq!(*op, BinOp::Sub),
                other => panic!("expected BinaryOp(Sub) in arg, got {other:?}"),
            },
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn multiplication_in_call_arg() {
        // n * fact(n - 1)  should parse as  n * (fact(n - 1))
        let m = parse("fun f(n: Int) -> Int:\n    n * f(n - 1)\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::BinaryOp { op, lhs, rhs } => {
                assert_eq!(*op, BinOp::Mul);
                assert!(matches!(lhs.as_ref(), Expr::Name(n) if n == "n"));
                assert!(matches!(rhs.as_ref(), Expr::Call { .. }));
            }
            other => panic!("expected BinaryOp(Mul), got {other:?}"),
        }
    }

    // ── binary operators: in if conditions ───────────────────────

    #[test]
    fn comparison_in_if_condition() {
        let m = parse("fun f(n: Int) -> Int:\n    if n <= 1:\n        1\n    else:\n        n\n    .\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::If { branches, .. } => {
                match &branches[0].0 {
                    Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::LtEq),
                    other => panic!("expected LtEq condition, got {other:?}"),
                }
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    #[test]
    fn eq_eq_in_elif_condition() {
        let m = parse("fun f(x: Int) -> Str:\n    if x < 0:\n        \"neg\"\n    elif x == 0:\n        \"zero\"\n    else:\n        \"pos\"\n    .\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::If { branches, .. } => {
                assert_eq!(branches.len(), 2);
                match &branches[0].0 {
                    Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::Lt),
                    other => panic!("expected Lt condition, got {other:?}"),
                }
                match &branches[1].0 {
                    Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::EqEq),
                    other => panic!("expected EqEq condition, got {other:?}"),
                }
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    // ── binary operators: string concat ──────────────────────────

    #[test]
    fn string_concat_with_plus() {
        let m = parse("fun f() -> Str:\n    me'first + \" \" + me'last\n.\n");
        let f = first_fun(&m);
        // Should be ((me'first + " ") + me'last) — left-associative
        match first_stmt_expr(&f.body) {
            Expr::BinaryOp { op, lhs, rhs } => {
                assert_eq!(*op, BinOp::Add);
                match lhs.as_ref() {
                    Expr::BinaryOp { op, lhs, rhs } => {
                        assert_eq!(*op, BinOp::Add);
                        assert!(matches!(lhs.as_ref(), Expr::Member { .. }));
                        assert!(matches!(rhs.as_ref(), Expr::String(s) if s == " "));
                    }
                    other => panic!("expected inner Add, got {other:?}"),
                }
                assert!(matches!(rhs.as_ref(), Expr::Member { .. }));
            }
            other => panic!("expected BinaryOp(Add), got {other:?}"),
        }
    }

    // ── binary operators: val binding ────────────────────────────

    #[test]
    fn operator_in_val_binding() {
        let m = parse("fun f(a: Int, b: Int) -> Int:\n    val total = a + b * 2\n    total\n.\n");
        let f = first_fun(&m);
        match &f.body[0] {
            Stmt::ValBind { name, value } => {
                assert_eq!(name, "total");
                match value {
                    Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::Add),
                    other => panic!("expected BinaryOp(Add), got {other:?}"),
                }
            }
            other => panic!("expected ValBind, got {other:?}"),
        }
    }

    // ── gt / gte operators ────────────────────────────────────────

    #[test]
    fn simple_gt() {
        let m = parse("fun f(x: Int) -> Bool:\n    x > 0\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::Gt),
            other => panic!("expected BinaryOp(Gt), got {other:?}"),
        }
    }

    #[test]
    fn simple_gte() {
        let m = parse("fun f(x: Int) -> Bool:\n    x >= 10\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::GtEq),
            other => panic!("expected BinaryOp(GtEq), got {other:?}"),
        }
    }

    #[test]
    fn gt_in_if_condition() {
        let m = parse("fun f(x: Int) -> Str:\n    if x > 100:\n        \"big\"\n    else:\n        \"small\"\n    .\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::If { branches, .. } => {
                assert!(matches!(&branches[0].0, Expr::BinaryOp { op: BinOp::Gt, .. }));
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    // ── unit literal ─────────────────────────────────────────────

    #[test]
    fn unit_literal_standalone() {
        let m = parse("fun f() -> Unit:\n    ()\n.\n");
        let f = first_fun(&m);
        assert!(matches!(first_stmt_expr(&f.body), Expr::Unit));
    }

    #[test]
    fn unit_literal_in_if_else() {
        let m = parse("fun f(x: Bool) -> Unit:\n    if x:\n        print(\"hi\")\n    else:\n        ()\n    .\n.\n");
        let f = first_fun(&m);
        match first_stmt_expr(&f.body) {
            Expr::If { else_branch, .. } => {
                let stmts = else_branch.as_ref().unwrap();
                assert!(matches!(first_stmt_expr(stmts), Expr::Unit));
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    // ── improved error messages ──────────────────────────────────

    #[test]
    fn error_message_if_missing_colon_is_contextual() {
        let msg = parse_err("fun f(x: Bool) -> Int:\n    if x\n        1\n    .\n.\n");
        assert!(msg.contains("after `if` condition"), "got: {msg}");
    }

    #[test]
    fn error_message_match_missing_colon_is_contextual() {
        let msg = parse_err("fun f(x: T) -> Int:\n    match x\n        a:\n            1\n    .\n.\n");
        assert!(msg.contains("after `match` scrutinee"), "got: {msg}");
    }

    #[test]
    fn error_message_fun_missing_colon_is_contextual() {
        let msg = parse_err("fun f()\n    x\n.\n");
        assert!(msg.contains("after function signature"), "got: {msg}");
    }

    #[test]
    fn error_message_expected_expression_is_clear() {
        let msg = parse_err("fun f() -> Int:\n    :\n.\n");
        assert!(msg.contains("expected expression"), "got: {msg}");
    }

    #[test]
    fn error_message_expected_pattern_is_clear() {
        let msg = parse_err("fun f(x: T) -> Int:\n    match x:\n        42:\n            1\n    .\n.\n");
        assert!(msg.contains("expected pattern"), "got: {msg}");
    }

    // ── error cases: unit literal ────────────────────────────────

    #[test]
    fn error_unit_literal_with_content() {
        let msg = parse_err("fun f() -> Unit:\n    (,)\n.\n");
        assert!(!msg.is_empty());
    }

    // ── error cases: named arguments ─────────────────────────────

    #[test]
    fn error_named_arg_missing_value() {
        let msg = parse_err("fun f() -> Unit:\n    g(key:)\n.\n");
        assert!(!msg.is_empty());
    }

    #[test]
    fn error_named_arg_missing_comma() {
        let msg = parse_err("fun f() -> Unit:\n    g(a: 1 b: 2)\n.\n");
        assert!(!msg.is_empty());
    }

    // ── error cases: operators in nested contexts ────────────────

    #[test]
    fn error_operator_in_match_arm_body_unterminated() {
        let msg = parse_err("fun f(x: T) -> Int:\n    match x:\n        a:\n            a +\n    .\n.\n");
        assert!(!msg.is_empty());
    }

    // ── full example: recursive factorial ────────────────────────

    #[test]
    fn factorial_full() {
        let m = parse(
            "fun fact(n: Int) -> Int:\n    if n <= 1:\n        1\n    else:\n        n * fact(n - 1)\n    .\n.\n",
        );
        let f = first_fun(&m);
        assert_eq!(f.name, "fact");
        match first_stmt_expr(&f.body) {
            Expr::If { branches, else_branch } => {
                // condition: n <= 1
                assert!(matches!(&branches[0].0, Expr::BinaryOp { op: BinOp::LtEq, .. }));
                // else body: n * fact(n - 1)
                let else_stmts = else_branch.as_ref().unwrap();
                match first_stmt_expr(else_stmts) {
                    Expr::BinaryOp { op, lhs, rhs } => {
                        assert_eq!(*op, BinOp::Mul);
                        assert!(matches!(lhs.as_ref(), Expr::Name(n) if n == "n"));
                        match rhs.as_ref() {
                            Expr::Call { callee, args } => {
                                assert!(matches!(callee.as_ref(), Expr::Name(n) if n == "fact"));
                                assert_eq!(args.len(), 1);
                                match &args[0] {
                                    Arg::Positional(Expr::BinaryOp { op, .. }) => {
                                        assert_eq!(*op, BinOp::Sub);
                                    }
                                    other => panic!("expected Sub in call arg, got {other:?}"),
                                }
                            }
                            other => panic!("expected Call, got {other:?}"),
                        }
                    }
                    other => panic!("expected BinaryOp(Mul), got {other:?}"),
                }
            }
            other => panic!("expected If, got {other:?}"),
        }
    }
}
