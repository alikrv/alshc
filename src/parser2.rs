// src/parser2.rs
use crate::control_flow::{
    BinOp, CompareOp, Condition, Expression, Statement, StringPart, UnOp, Value,
};
use crate::lexer::{Lexer, StrPart, Token, TokenKind};

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "parse error at {}:{}: {}",
            self.line, self.col, self.message
        )
    }
}

type PResult<T> = Result<T, ParseError>;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    pub has_main: bool,
    pub just_run: bool,
    pub main_function_name: Option<String>,
    pub noffi: bool,
    pub use_stdlib: bool,
}

impl Parser {
    pub fn from_source(src: &str) -> PResult<Self> {
        let tokens = Lexer::new(src).tokenize().map_err(|e| ParseError {
            message: e.message,
            line: e.line,
            col: e.col,
        })?;
        Ok(Parser {
            tokens,
            pos: 0,
            has_main: false,
            just_run: false,
            main_function_name: None,
            noffi: false,
            use_stdlib: false,
        })
    }

    // ---- token stream helpers ----

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }
    fn peek_tok(&self) -> &Token {
        &self.tokens[self.pos]
    }
    fn advance(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        t
    }
    fn check(&self, kind: &TokenKind) -> bool {
        self.peek() == kind
    }
    fn at_eof(&self) -> bool {
        matches!(self.peek(), TokenKind::Eof)
    }

    fn err(&self, msg: impl Into<String>) -> ParseError {
        let t = self.peek_tok();
        ParseError {
            message: msg.into(),
            line: t.line,
            col: t.col,
        }
    }

    fn expect(&mut self, kind: TokenKind) -> PResult<Token> {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(&kind) {
            Ok(self.advance())
        } else {
            Err(self.err(format!("expected {:?}, found {:?}", kind, self.peek())))
        }
    }

    fn eat_ident(&mut self) -> PResult<String> {
        match self.peek().clone() {
            TokenKind::Ident(s) => {
                self.advance();
                Ok(s)
            }
            other => Err(self.err(format!("expected identifier, found {:?}", other))),
        }
    }

    // ---- top level ----

    pub fn parse(&mut self) -> PResult<Vec<Statement>> {
        let mut statements = Vec::new();
        while !self.at_eof() {
            if let TokenKind::Directive(d) = self.peek().clone() {
                self.advance();
                self.handle_directive(&d)?;
                continue;
            }
            statements.push(self.parse_statement()?);
        }
        Ok(statements)
    }

    fn handle_directive(&mut self, raw: &str) -> PResult<()> {
        // Preprocessor directives are handled by the external preprocessor.
        // The compiler should not act on them. Ignore any directive tokens.
        let _ = raw;
        Ok(())
    }

    fn peek_function_name(&self) -> Option<String> {
        // tokens: Function Ident(name) LParen ...
        match self.tokens.get(self.pos + 1)?.kind.clone() {
            TokenKind::Ident(n) => Some(n),
            _ => None,
        }
    }

    // ---- statements ----

    fn parse_statement(&mut self) -> PResult<Statement> {
        match self.peek().clone() {
            TokenKind::Directive(d) => {
                self.advance();
                self.handle_directive(&d)?;
                self.parse_statement()
            }
            TokenKind::MainFunction => {
                // preprocessor marks main functions using MAIN_FUNCTION
                // let parse_function handle consuming the token and marking has_main
                self.parse_function()
            }
            TokenKind::Let => self.parse_let(),
            TokenKind::Function => self.parse_function(),
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for_or_cfor(),
            TokenKind::Foreach => self.parse_foreach(),
            TokenKind::Loop => self.parse_loop(),
            TokenKind::Break => {
                self.advance();
                Ok(Statement::Break { condition: None })
            }
            TokenKind::Continue => {
                self.advance();
                Ok(Statement::Continue)
            }
            TokenKind::Return => self.parse_return(),
            TokenKind::Try => self.parse_try(),
            TokenKind::Struct => self.parse_struct(),
            TokenKind::Enum => self.parse_enum(),
            TokenKind::Scan => self.parse_scan(),
            TokenKind::Switch => self.parse_switch(),
            TokenKind::Chain => self.parse_chain(),
            TokenKind::Dollar => self.parse_assignment(),
            TokenKind::LBrace => {
                // bare block as a statement (scope only, value discarded)
                let stmts = self.parse_block()?;
                Ok(Statement::Expression(Expression::Block(stmts)))
            }
            _ => {
                let expr = self.parse_expr(0)?;
                self.skip_semi();
                Ok(Statement::Expression(expr))
            }
        }
    }

    fn skip_semi(&mut self) {
        if self.check(&TokenKind::Semi) {
            self.advance();
        }
    }

    fn parse_block(&mut self) -> PResult<Vec<Statement>> {
        self.expect(TokenKind::LBrace)?;
        let mut statements = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            if self.at_eof() {
                return Err(self.err("expected matching '}'"));
            }
            if let TokenKind::Directive(d) = self.peek().clone() {
                self.advance();
                self.handle_directive(&d)?;
                continue;
            }
            statements.push(self.parse_statement()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(statements)
    }

    fn parse_let(&mut self) -> PResult<Statement> {
        self.expect(TokenKind::Let)?;
        if self.check(&TokenKind::Const) {
            self.advance();
        } // `let const x = ...`
        let name = self.eat_ident()?;
        self.expect(TokenKind::Eq)?;
        let value = if self.check(&TokenKind::LBrace) {
            Expression::Block(self.parse_block()?)
        } else {
            self.parse_expr(0)?
        };
        self.skip_semi();
        Ok(Statement::Let { name, value })
    }

    fn parse_assignment(&mut self) -> PResult<Statement> {
        self.expect(TokenKind::Dollar)?;
        let mut name = self.eat_ident()?;
        while self.check(&TokenKind::Dot) {
            self.advance();
            name.push('.');
            name.push_str(&self.eat_ident()?);
        }
        self.expect(TokenKind::Eq)?;
        let value = if self.check(&TokenKind::LBrace) {
            Expression::Block(self.parse_block()?)
        } else {
            self.parse_expr(0)?
        };
        self.skip_semi();
        Ok(Statement::Let { name, value }) // reuses Let as "bind/rebind" like your original codegen does
    }

    fn parse_function(&mut self) -> PResult<Statement> {
        // Accept either FUNCTION or MAIN_FUNCTION (preprocessor may emit MAIN_FUNCTION)
        let mut marked_main = false;
        match self.peek().clone() {
            TokenKind::Function => { self.advance(); }
            TokenKind::MainFunction => { self.advance(); self.has_main = true; marked_main = true; }
            _ => return Err(self.err("expected function declaration")),
        }
        let name = self.eat_ident()?;
        if marked_main {
            self.main_function_name = Some(name.clone());
        }
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        while !self.check(&TokenKind::RParen) {
            // supports both `name` and `type name` forms
            let first = self.eat_ident()?;
            if let TokenKind::Ident(second) = self.peek().clone() {
                self.advance();
                params.push(second); // `int x` -> keep param name, drop type for now
                let _ = first;
            } else {
                params.push(first);
            }
            if self.check(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(TokenKind::RParen)?;

        // optional return type: `int`, `str`, `noret`, etc. — consume one ident/Noret if present
        if !self.check(&TokenKind::LBrace) {
            match self.peek().clone() {
                TokenKind::Noret => {
                    self.advance();
                }
                TokenKind::Ident(_) => {
                    self.advance();
                }
                _ => {}
            }
        }

        let body = self.parse_block()?;
        Ok(Statement::FunctionDef { name, params, body })
    }

    fn parse_if(&mut self) -> PResult<Statement> {
        self.expect(TokenKind::If)?;
        let condition = self.parse_condition()?;
        let then_block = self.parse_block()?;

        let mut elif_blocks = Vec::new();
        let mut else_block = None;

        loop {
            match self.peek() {
                TokenKind::Elif => {
                    self.advance();
                    let cond = self.parse_condition()?;
                    let body = self.parse_block()?;
                    elif_blocks.push((cond, body));
                }
                TokenKind::Else => {
                    self.advance();
                    if self.check(&TokenKind::If) {
                        // `else if` sugar -> nested elif
                        self.advance();
                        let cond = self.parse_condition()?;
                        let body = self.parse_block()?;
                        elif_blocks.push((cond, body));
                    } else {
                        else_block = Some(self.parse_block()?);
                        break;
                    }
                }
                _ => break,
            }
        }

        Ok(Statement::If {
            condition,
            then_block,
            elif_blocks,
            else_block,
        })
    }

    fn parse_while(&mut self) -> PResult<Statement> {
        self.expect(TokenKind::While)?;
        let condition = self.parse_condition()?;
        let body = self.parse_block()?;
        Ok(Statement::While { condition, body })
    }

    fn parse_for_or_cfor(&mut self) -> PResult<Statement> {
        self.expect(TokenKind::For)?;
        if self.check(&TokenKind::LParen) {
            self.advance();
            let init = if self.check(&TokenKind::Comma) {
                None
            } else {
                Some(self.parse_init_clause()?)
            };
            self.expect(TokenKind::Comma)?;
            let condition = self.parse_condition()?;
            self.expect(TokenKind::Comma)?;
            let update = if self.check(&TokenKind::RParen) {
                None
            } else {
                Some(self.parse_update_clause()?)
            };
            self.expect(TokenKind::RParen)?;
            let body = self.parse_block()?;
            Ok(Statement::ForLoop {
                init,
                condition,
                update,
                body,
            })
        } else {
            let var = self.eat_ident()?;
            self.expect(TokenKind::In)?;
            let mut items = vec![self.parse_expr(0)?];
            while !self.check(&TokenKind::LBrace) {
                items.push(self.parse_expr(0)?);
            }
            let body = self.parse_block()?;
            Ok(Statement::For { var, items, body })
        }
    }

    // `let x = 0` or `$x = 0` inside a for(...) header
    fn parse_init_clause(&mut self) -> PResult<Expression> {
        if self.check(&TokenKind::Let) {
            self.advance();
            self.eat_ident()?;
            self.expect(TokenKind::Eq)?;
            self.parse_expr(0)
        } else if self.check(&TokenKind::Dollar) {
            self.advance();
            self.eat_ident()?;
            self.expect(TokenKind::Eq)?;
            self.parse_expr(0)
        } else {
            self.parse_expr(0)
        }
    }
    fn parse_update_clause(&mut self) -> PResult<Expression> {
        self.parse_init_clause()
    }

    fn parse_foreach(&mut self) -> PResult<Statement> {
        self.expect(TokenKind::Foreach)?;
        let var = self.eat_ident()?;
        self.expect(TokenKind::In)?;
        let iterable = self.parse_expr(0)?;
        let body = self.parse_block()?;
        Ok(Statement::Foreach {
            var,
            iterable,
            body,
        })
    }

    fn parse_loop(&mut self) -> PResult<Statement> {
        self.expect(TokenKind::Loop)?;
        let mut count = None;
        let mut interval = None;
        match self.peek().clone() {
            TokenKind::Count => {
                self.advance();
                if let TokenKind::Number(n) = self.peek().clone() {
                    self.advance();
                    count = Some(n as u64);
                } else {
                    return Err(self.err("expected number after 'count'"));
                }
            }
            TokenKind::Interval => {
                self.advance();
                if let TokenKind::Number(n) = self.peek().clone() {
                    self.advance();
                    interval = Some(n as u64);
                } else {
                    return Err(self.err("expected number after 'interval'"));
                }
            }
            _ => {}
        }
        let body = self.parse_block()?;
        Ok(Statement::Loop {
            count,
            interval,
            body,
        })
    }

    fn parse_return(&mut self) -> PResult<Statement> {
        self.expect(TokenKind::Return)?;
        let value = if self.check(&TokenKind::Semi) || self.check(&TokenKind::RBrace) {
            None
        } else {
            Some(self.parse_expr(0)?)
        };
        self.skip_semi();
        Ok(Statement::Return { value })
    }

    fn parse_try(&mut self) -> PResult<Statement> {
        self.expect(TokenKind::Try)?;
        let try_block = self.parse_block()?;
        let catch_block = if self.check(&TokenKind::Catch) {
            self.advance();
            Some(self.parse_block()?)
        } else {
            None
        };
        Ok(Statement::Try {
            try_block,
            catch_block,
        })
    }

    fn parse_struct(&mut self) -> PResult<Statement> {
        self.expect(TokenKind::Struct)?;
        let name = self.eat_ident()?;
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            let field_name = self.eat_ident()?;
            self.expect(TokenKind::Colon)?;
            let _ty = self.eat_ident()?; // type name, not tracked yet
            fields.push(field_name);
            if self.check(&TokenKind::Comma) {
                self.advance();
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Statement::StructDef { name, fields })
    }

    fn parse_enum(&mut self) -> PResult<Statement> {
        self.expect(TokenKind::Enum)?;
        let name = self.eat_ident()?;
        self.expect(TokenKind::LBrace)?;
        let mut variants = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            variants.push(self.eat_ident()?);
            if self.check(&TokenKind::Comma) {
                self.advance();
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Statement::EnumDef { name, variants })
    }

    fn parse_scan(&mut self) -> PResult<Statement> {
        self.expect(TokenKind::Scan)?;
        let expr = if self.check(&TokenKind::Of) {
            None
        } else {
            Some(self.parse_expr(0)?)
        };
        self.expect(TokenKind::Of)?;
        let enum_type = self.eat_ident()?;
        self.expect(TokenKind::LBrace)?;
        let mut branches = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            let label = self.eat_ident()?;
            self.expect(TokenKind::Colon)?;
            let stmt = self.parse_statement()?;
            branches.push((label, vec![stmt]));
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Statement::Scan {
            expr,
            enum_type,
            branches,
        })
    }

    fn parse_switch(&mut self) -> PResult<Statement> {
        self.expect(TokenKind::Switch)?;
        self.expect(TokenKind::On)?;
        let expr = self.parse_expr(0)?;
        self.expect(TokenKind::LBrace)?;
        let mut branches = Vec::new();
        let mut default_branch = None;
        while !self.check(&TokenKind::RBrace) {
            if self.check(&TokenKind::Default) {
                self.advance();
                self.expect(TokenKind::Colon)?;
                default_branch = Some(vec![self.parse_statement()?]);
            } else {
                let label = self.eat_ident()?;
                self.expect(TokenKind::Colon)?;
                let stmt = self.parse_statement()?;
                branches.push((label, vec![stmt]));
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Statement::Switch {
            expr,
            branches,
            default_branch,
        })
    }

    fn parse_chain(&mut self) -> PResult<Statement> {
        self.expect(TokenKind::Chain)?;
        self.expect(TokenKind::LBrace)?;
        let mut steps = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            steps.push(self.parse_expr(0)?);
            self.skip_semi();
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Statement::Chain { steps })
    }

    // ---- conditions (thin wrapper around expr parsing for `is`/`is not`/&&/||) ----

    fn parse_condition(&mut self) -> PResult<Condition> {
        let paren = self.check(&TokenKind::LParen);
        if paren {
            self.advance();
        }
        let cond = self.parse_condition_or()?;
        if paren {
            self.expect(TokenKind::RParen)?;
        }
        Ok(cond)
    }

    fn parse_condition_or(&mut self) -> PResult<Condition> {
        let mut left = self.parse_condition_and()?;
        while self.check(&TokenKind::OrOr) {
            self.advance();
            let right = self.parse_condition_and()?;
            left = Condition::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_condition_and(&mut self) -> PResult<Condition> {
        let mut left = self.parse_condition_atom()?;
        while self.check(&TokenKind::AndAnd) {
            self.advance();
            let right = self.parse_condition_atom()?;
            left = Condition::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_condition_atom(&mut self) -> PResult<Condition> {
        if self.check(&TokenKind::LParen) {
            self.advance();
            let inner = self.parse_condition_or()?;
            self.expect(TokenKind::RParen)?;
            return Ok(inner);
        }
        let left = self.parse_expr(0)?;
        let op = match self.peek() {
            TokenKind::EqEq => Some(CompareOp::Eq),
            TokenKind::NotEq => Some(CompareOp::Ne),
            TokenKind::Lt => Some(CompareOp::Lt),
            TokenKind::Gt => Some(CompareOp::Gt),
            TokenKind::Le => Some(CompareOp::Le),
            TokenKind::Ge => Some(CompareOp::Ge),
            _ => None,
        };
        if let Some(op) = op {
            self.advance();
            let right = self.parse_expr(0)?;
            return Ok(Condition::Compare(left, op, right));
        }
        Ok(Condition::Command(left))
    }

    // ---- expressions: Pratt parser ----

    fn parse_expr(&mut self, min_bp: u8) -> PResult<Expression> {
        let mut lhs = self.parse_unary()?;
        loop {
            let Some((op, l_bp, r_bp)) = self.peek_binop() else {
                break;
            };
            if l_bp < min_bp {
                break;
            }
            self.advance();
            let rhs = self.parse_expr(r_bp)?;
            lhs = Expression::BinaryOp(Box::new(lhs), op, Box::new(rhs));
        }
        Ok(lhs)
    }

    fn peek_binop(&self) -> Option<(BinOp, u8, u8)> {
        Some(match self.peek() {
            TokenKind::Plus => (BinOp::Add, 7, 8),
            TokenKind::Minus => (BinOp::Sub, 7, 8),
            TokenKind::Star => (BinOp::Mul, 9, 10),
            TokenKind::Slash => (BinOp::Div, 9, 10),
            _ => return None,
        })
    }

    fn parse_unary(&mut self) -> PResult<Expression> {
        match self.peek() {
            TokenKind::Minus => {
                self.advance();
                Ok(Expression::UnaryOp(
                    UnOp::Neg,
                    Box::new(self.parse_unary()?),
                ))
            }
            TokenKind::Not => {
                self.advance();
                Ok(Expression::UnaryOp(
                    UnOp::Not,
                    Box::new(self.parse_unary()?),
                ))
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> PResult<Expression> {
        let expr = self.parse_primary()?;
        loop {
            match self.peek() {
                TokenKind::LParen => {
                    // only valid as a call if primary was a bare name-like expr;
                    // callee-by-name calls are handled in parse_primary for
                    // Ident/std::/c:: forms, so this covers chained calls if ever added.
                    break;
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> PResult<Expression> {
        match self.peek().clone() {
            TokenKind::Number(n) => {
                self.advance();
                Ok(Expression::Literal(Value::Number(n)))
            }
            TokenKind::Float(f) => {
                self.advance();
                Ok(Expression::Literal(Value::Float(f)))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expression::Literal(Value::Bool(true)))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expression::Literal(Value::Bool(false)))
            }
            TokenKind::RawStr(s) => {
                self.advance();
                Ok(Expression::Literal(Value::String(s)))
            }
            TokenKind::Char(c) => {
                self.advance();
                Ok(Expression::Literal(Value::String(c.to_string())))
            }
            TokenKind::Str(parts) => {
                self.advance();
                if parts.len() == 1 {
                    if let StrPart::Literal(s) = &parts[0] {
                        return Ok(Expression::Literal(Value::String(s.clone())));
                    }
                }
                let converted = parts
                    .into_iter()
                    .map(|p| match p {
                        StrPart::Literal(s) => StringPart::Literal(s),
                        StrPart::Var(name) => StringPart::Interpolation(Expression::Variable(name)),
                    })
                    .collect();
                Ok(Expression::StringInterpolation(converted))
            }
            TokenKind::Dollar => {
                self.advance();
                let mut name = self.eat_ident()?;
                while self.check(&TokenKind::Dot) {
                    self.advance();
                    name.push('.');
                    name.push_str(&self.eat_ident()?);
                }
                Ok(Expression::Variable(name))
            }
            TokenKind::At => {
                self.advance();
                Ok(Expression::Variable("@".to_string()))
            } // chain placeholder
            TokenKind::LParen => {
                self.advance();
                let inner = self.parse_expr(0)?;
                self.expect(TokenKind::RParen)?;
                Ok(inner)
            }
            TokenKind::LBracket => {
                self.advance();
                let mut items = Vec::new();
                while !self.check(&TokenKind::RBracket) {
                    items.push(self.parse_expr(0)?);
                    if self.check(&TokenKind::Comma) {
                        self.advance();
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Expression::Array(items))
            }
            TokenKind::LBrace => Ok(Expression::Block(self.parse_block()?)),
            TokenKind::Ident(name) => {
                self.advance();

                // c::foo(...) / std::foo(...) namespaced calls arrive as one
                // Ident token because the lexer folds `::` into identifiers.
                if let TokenKind::LParen = self.peek() {
                    self.advance();
                    let args = self.parse_call_args()?;
                    if let Some(rest) = name.strip_prefix("c::") {
                        return Ok(Expression::CCall(rest.to_string(), args));
                    }
                    return Ok(Expression::FunctionCall(name, args));
                }

                // struct literal: Ident { field: expr ... }
                if self.check(&TokenKind::LBrace) {
                    self.advance();
                    let mut fields = Vec::new();
                    while !self.check(&TokenKind::RBrace) {
                        let fname = self.eat_ident()?;
                        self.expect(TokenKind::Colon)?;
                        let fval = self.parse_expr(0)?;
                        fields.push((fname, fval));
                        if self.check(&TokenKind::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::RBrace)?;
                    return Ok(Expression::StructLiteral(name, fields));
                }

                // enum literal: EnumType.variant
                if self.check(&TokenKind::Dot) {
                    self.advance();
                    let variant = self.eat_ident()?;
                    return Ok(Expression::EnumLiteral(name, variant));
                }

                Ok(Expression::Variable(name)) // bare identifier used as value (e.g. args array)
            }
            other => Err(self.err(format!("unexpected token in expression: {:?}", other))),
        }
    }

    fn parse_call_args(&mut self) -> PResult<Vec<Expression>> {
        let mut args = Vec::new();
        while !self.check(&TokenKind::RParen) {
            args.push(self.parse_expr(0)?);
            if self.check(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(TokenKind::RParen)?;
        Ok(args)
    }
}
