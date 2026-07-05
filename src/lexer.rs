// src/lexer.rs
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // literals
    Ident(String),
    Number(i64),
    Float(f64),
    Str(Vec<StrPart>), // interpolated string, pre-split at lex time
    RawStr(String),    // single-quoted, no interpolation
    Char(char),

    // keywords
    Let,
    Const,
    Function,
    Fn,
    Return,
    If,
    Elif,
    Else,
    While,
    Loop,
    For,
    Foreach,
    In,
    Break,
    Continue,
    Try,
    Catch,
    Struct,
    Enum,
    Scan,
    Switch,
    Default,
    Of,
    On,
    Count,
    Interval,
    Chain,
    True,
    False,
    Noret,
    MainFunction,

    // punctuation
    Dollar,
    Comma,
    Colon,
    Semi,
    Dot,
    Ellipsis,
    Arrow,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    // operators
    Plus,
    Minus,
    Star,
    Slash,
    Eq,
    EqEq,
    NotEq,
    Lt,
    Gt,
    Le,
    Ge,
    AndAnd,
    OrOr,
    Not,
    At, // '@' preprocessor / chain placeholder

    Directive(String), // '@main', '@stdlib', etc, captured whole

    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StrPart {
    Literal(String),
    Var(String), // raw name after '$', dotted paths allowed
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct LexError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "lex error at {}:{}: {}",
            self.line, self.col, self.message
        )
    }
}

pub struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer {
            src: src.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            let (line, col) = (self.line, self.col);
            let Some(c) = self.peek() else {
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    line,
                    col,
                });
                break;
            };

            let kind = match c {
                '"' => self.lex_interpolated_string()?,
                '\'' => self.lex_raw_or_char()?,
                '$' => {
                    self.advance();
                    TokenKind::Dollar
                }
                '(' => {
                    self.advance();
                    TokenKind::LParen
                }
                ')' => {
                    self.advance();
                    TokenKind::RParen
                }
                '{' => {
                    self.advance();
                    TokenKind::LBrace
                }
                '}' => {
                    self.advance();
                    TokenKind::RBrace
                }
                '[' => {
                    self.advance();
                    TokenKind::LBracket
                }
                ']' => {
                    self.advance();
                    TokenKind::RBracket
                }
                ',' => {
                    self.advance();
                    TokenKind::Comma
                }
                ':' => {
                    self.advance();
                    TokenKind::Colon
                }
                ';' => {
                    self.advance();
                    TokenKind::Semi
                }
                '.' => {
                    // Recognize '...' as Ellipsis token
                    if self.peek_at(1) == Some('.') && self.peek_at(2) == Some('.') {
                        self.advance(); self.advance(); self.advance();
                        TokenKind::Ellipsis
                    } else {
                        self.advance();
                        TokenKind::Dot
                    }
                }
                '+' => {
                    self.advance();
                    TokenKind::Plus
                }
                '-' => {
                    self.advance();
                    if self.peek() == Some('>') {
                        self.advance();
                        TokenKind::Arrow
                    } else {
                        TokenKind::Minus
                    }
                }
                '*' => {
                    self.advance();
                    TokenKind::Star
                }
                '/' => {
                    self.advance();
                    TokenKind::Slash
                }
                '=' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::EqEq
                    } else {
                        TokenKind::Eq
                    }
                }
                '!' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::NotEq
                    } else {
                        TokenKind::Not
                    }
                }
                '<' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::Le
                    } else {
                        TokenKind::Lt
                    }
                }
                '>' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::Ge
                    } else {
                        TokenKind::Gt
                    }
                }
                '&' => {
                    self.advance();
                    if self.peek() == Some('&') {
                        self.advance();
                        TokenKind::AndAnd
                    } else {
                        return Err(self.err("unexpected '&'"));
                    }
                }
                '|' => {
                    self.advance();
                    if self.peek() == Some('|') {
                        self.advance();
                        TokenKind::OrOr
                    } else {
                        return Err(self.err("unexpected '|' (shell pipes not lexed here)"));
                    }
                }
                '@' => self.lex_directive_or_at()?,
                c if c.is_ascii_digit() => self.lex_number()?,
                c if c.is_alphabetic() || c == '_' => self.lex_ident_or_keyword(),
                _ => return Err(self.err(&format!("unexpected character '{}'", c))),
            };

            tokens.push(Token { kind, line, col });
        }
        Ok(tokens)
    }

    fn peek(&self) -> Option<char> {
        self.src.get(self.pos).map(|&b| b as char)
    }
    fn peek_at(&self, off: usize) -> Option<char> {
        self.src.get(self.pos + off).map(|&b| b as char)
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn err(&self, msg: &str) -> LexError {
        LexError {
            message: msg.to_string(),
            line: self.line,
            col: self.col,
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                Some('#') => {
                    // '#!' shebang or '#' comment: skip to end of line either way
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                Some('/') if self.peek_at(1) == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    fn lex_number(&mut self) -> Result<TokenKind, LexError> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.advance();
        }
        let mut is_float = false;
        if self.peek() == Some('.') && matches!(self.peek_at(1), Some(c) if c.is_ascii_digit()) {
            is_float = true;
            self.advance();
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.advance();
            }
        }
        let text = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
        if is_float {
            text.parse::<f64>()
                .map(TokenKind::Float)
                .map_err(|_| self.err("invalid float literal"))
        } else {
            text.parse::<i64>()
                .map(TokenKind::Number)
                .map_err(|_| self.err("invalid integer literal"))
        }
    }

    fn lex_ident_or_keyword(&mut self) -> TokenKind {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_') {
            self.advance();
        }
        // allow `::` inside identifiers for std::/c:: namespacing
        while self.peek() == Some(':') && self.peek_at(1) == Some(':') {
            self.advance();
            self.advance();
            while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_') {
                self.advance();
            }
        }
        let text = std::str::from_utf8(&self.src[start..self.pos])
            .unwrap()
            .to_string();
        match text.to_uppercase().as_str() {
            "LET" => TokenKind::Let,
            "CONST" => TokenKind::Const,
            "FUNCTION" | "FN" => TokenKind::Function,
            "RETURN" => TokenKind::Return,
            "IF" => TokenKind::If,
            "ELIF" => TokenKind::Elif,
            "ELSE" => TokenKind::Else,
            "WHILE" => TokenKind::While,
            "LOOP" => TokenKind::Loop,
            "FOR" => TokenKind::For,
            "FOREACH" => TokenKind::Foreach,
            "IN" => TokenKind::In,
            "BREAK" => TokenKind::Break,
            "CONTINUE" => TokenKind::Continue,
            "TRY" => TokenKind::Try,
            "CATCH" => TokenKind::Catch,
            "STRUCT" => TokenKind::Struct,
            "ENUM" => TokenKind::Enum,
            "SCAN" => TokenKind::Scan,
            "SWITCH" => TokenKind::Switch,
            "DEFAULT" => TokenKind::Default,
            "OF" => TokenKind::Of,
            "ON" => TokenKind::On,
            "COUNT" => TokenKind::Count,
            "INTERVAL" => TokenKind::Interval,
            "CHAIN" => TokenKind::Chain,
            "TRUE" => TokenKind::True,
            "FALSE" => TokenKind::False,
            "NORET" => TokenKind::Noret,
            "MAIN_FUNCTION" => TokenKind::MainFunction,
            _ => TokenKind::Ident(text),
        }
    }

    // Interpolated double-quoted string: split into Literal/Var parts at lex time.
    fn lex_interpolated_string(&mut self) -> Result<TokenKind, LexError> {
        self.advance(); // consume opening "
        let mut parts = Vec::new();
        let mut buf = String::new();

        loop {
            match self.peek() {
                None => return Err(self.err("unterminated string literal")),
                Some('"') => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    self.advance();
                    match self.advance() {
                        Some('n') => buf.push('\n'),
                        Some('t') => buf.push('\t'),
                        Some('"') => buf.push('"'),
                        Some('\\') => buf.push('\\'),
                        Some('$') => buf.push('$'),
                        Some(other) => buf.push(other),
                        None => return Err(self.err("unterminated escape sequence")),
                    }
                }
                Some('$') => {
                    self.advance();
                    if !buf.is_empty() {
                        parts.push(StrPart::Literal(std::mem::take(&mut buf)));
                    }
                    let mut name = String::new();
                    while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_' || c == '.')
                    {
                        name.push(self.advance().unwrap());
                    }
                    if name.is_empty() {
                        buf.push('$'); // bare '$', not a var
                    } else {
                        parts.push(StrPart::Var(name));
                    }
                }
                Some(c) => {
                    buf.push(c);
                    self.advance();
                }
            }
        }
        if !buf.is_empty() || parts.is_empty() {
            parts.push(StrPart::Literal(buf));
        }
        Ok(TokenKind::Str(parts))
    }

    // Single-quoted: raw string, OR a char literal if exactly one char e.g. 'a'
    fn lex_raw_or_char(&mut self) -> Result<TokenKind, LexError> {
        self.advance(); // consume opening '
        let mut buf = String::new();
        loop {
            match self.peek() {
                None => return Err(self.err("unterminated raw string/char literal")),
                Some('\'') => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    self.advance();
                    match self.advance() {
                        Some('n') => buf.push('\n'),
                        Some('\'') => buf.push('\''),
                        Some('\\') => buf.push('\\'),
                        Some(other) => buf.push(other),
                        None => return Err(self.err("unterminated escape sequence")),
                    }
                }
                Some(c) => {
                    buf.push(c);
                    self.advance();
                }
            }
        }
        if buf.chars().count() == 1 {
            Ok(TokenKind::Char(buf.chars().next().unwrap()))
        } else {
            Ok(TokenKind::RawStr(buf))
        }
    }

    // '@main', '@justrunit', '@define NAME val' etc. We capture the whole
    // directive line as text; the preprocessor stage (run before parsing)
    // consumes these, so the parser proper rarely sees them.
    fn lex_directive_or_at(&mut self) -> Result<TokenKind, LexError> {
        self.advance(); // consume '@'
                        // bare '@' (chain placeholder) has no identifier following
        if !matches!(self.peek(), Some(c) if c.is_alphabetic() || c == '_') {
            return Ok(TokenKind::At);
        }
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_') {
            self.advance();
        }
        let name = std::str::from_utf8(&self.src[start..self.pos])
            .unwrap()
            .to_string();
        // capture rest of line as directive argument text (e.g. @define NAME value)
        let rest_start = self.pos;
        while matches!(self.peek(), Some(c) if c != '\n') {
            self.advance();
        }
        let rest = std::str::from_utf8(&self.src[rest_start..self.pos])
            .unwrap()
            .trim()
            .to_string();
        let full = if rest.is_empty() {
            format!("@{}", name)
        } else {
            format!("@{} {}", name, rest)
        };
        Ok(TokenKind::Directive(full))
    }
}
