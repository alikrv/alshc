// src/control_flow.rs
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Number(i64),
    Float(f64),
    Bool(bool),
    List(Vec<String>),
    Struct(String, HashMap<String, Value>),
    Enum(String, String),
}

impl Value {
    pub fn as_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty() && s != "false" && s != "0",
            Value::List(items) => !items.is_empty(),
            Value::Struct(_, fields) => !fields.is_empty(),
            Value::Enum(_, _) => true,
        }
    }

    pub fn as_number(&self) -> Option<i64> {
        match self {
            Value::Number(n) => Some(*n),
            Value::Float(f) => Some(*f as i64),
            Value::String(s) => s.parse().ok(),
            Value::Bool(b) => Some(if *b { 1 } else { 0 }),
            Value::List(_) => None,
            Value::Struct(_, _) => None,
            Value::Enum(_, _) => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Number(n) => Some(*n as f64),
            Value::String(s) => s.parse().ok(),
            Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            Value::List(_) => None,
            Value::Struct(_, _) => None,
            Value::Enum(_, _) => None,
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Bool(b) => if *b { "true".to_string() } else { "false".to_string() },
            Value::List(items) => items.join(" "),
            Value::Struct(type_name, fields) => {
                let mut parts: Vec<String> = Vec::new();
                for (key, value) in fields {
                    parts.push(format!("{}: {}", key, value.as_string()));
                }
                format!("{} {{ {} }}", type_name, parts.join(" "))
            }
            Value::Enum(type_name, variant) => format!("{}.{}", type_name, variant),
        }
    }

    pub fn size_in_memory(&self) -> usize {
        match self {
            Value::String(s) => std::mem::size_of::<String>() + s.len(),
            Value::Number(_) => std::mem::size_of::<i64>(),
            Value::Float(_) => std::mem::size_of::<f64>(),
            Value::Bool(_) => std::mem::size_of::<bool>(),
            Value::List(items) => {
                let mut size = std::mem::size_of::<Vec<String>>() + items.len() * std::mem::size_of::<String>();
                for item in items {
                    size += std::mem::size_of::<String>() + item.len();
                }
                size
            }
            Value::Struct(type_name, fields) => {
                let mut size = std::mem::size_of::<String>() + type_name.len();
                size += std::mem::size_of::<std::collections::HashMap<String, Value>>();
                size += fields.len() * (std::mem::size_of::<String>() + std::mem::size_of::<Value>());
                for (key, value) in fields {
                    size += std::mem::size_of::<String>() + key.len();
                    size += value.size_in_memory();
                }
                size
            }
            Value::Enum(type_name, variant) => {
                std::mem::size_of::<String>() * 2 + type_name.len() + variant.len()
            }
        }
    }

    pub fn as_list(&self) -> Option<&Vec<String>> {
        match self {
            Value::List(items) => Some(items),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expression {
    Literal(Value),
    Variable(String),
    BinaryOp(Box<Expression>, BinOp, Box<Expression>),
    UnaryOp(UnOp, Box<Expression>),
    FunctionCall(String, Vec<Expression>),
    CCall(String, Vec<Expression>),
    Block(Vec<Statement>),
    StringInterpolation(Vec<StringPart>),
    Array(Vec<Expression>),
    StructLiteral(String, Vec<(String, Expression)>),
    EnumLiteral(String, String),
    Chain(Vec<Expression>),
}

#[derive(Debug, Clone)]
pub enum StringPart {
    Literal(String),
    Interpolation(Expression),
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub enum UnOp {
    Not,
    Neg,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Command(String),
    Let {
        name: String,
        value: Expression,
    },
    If {
        condition: Condition,
        then_block: Vec<Statement>,
        elif_blocks: Vec<(Condition, Vec<Statement>)>,
        else_block: Option<Vec<Statement>>,
    },
    While {
        condition: Condition,
        body: Vec<Statement>,
    },
    For {
        var: String,
        items: Vec<Expression>,
        body: Vec<Statement>,
    },
    Loop {
        count: Option<u64>,
        interval: Option<u64>,
        body: Vec<Statement>,
    },
    Break {
        condition: Option<Condition>,
    },
    Continue,
    Try {
        try_block: Vec<Statement>,
        catch_block: Option<Vec<Statement>>,
    },
    FunctionDef {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
    },
    Foreach {
        var: String,
        iterable: Expression,
        body: Vec<Statement>,
    },
    ForLoop {
        init: Option<Expression>,
        condition: Condition,
        update: Option<Expression>,
        body: Vec<Statement>,
    },
    Return {
        value: Option<Expression>,
    },
    Expression(Expression),
    Chain {
        steps: Vec<Expression>,
    },
    StructDef {
        name: String,
        fields: Vec<String>,
    },
    EnumDef {
        name: String,
        variants: Vec<String>,
    },
    Scan {
        expr: Option<Expression>,
        enum_type: String,
        branches: Vec<(String, Vec<Statement>)>,
    },
    Switch {
        expr: Expression,
        branches: Vec<(String, Vec<Statement>)>,
        default_branch: Option<Vec<Statement>>,
    },
}

#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub params: Vec<String>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Condition {
    Command(Expression),
    Is(Expression, Expression),
    IsNot(Expression, Expression),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Compare(Expression, CompareOp, Expression),
}

#[derive(Debug, Clone)]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

pub struct ControlFlowParser {
    lines: Vec<String>,
    pos: usize,
    pub has_main: bool,
    pub just_run: bool,
    pub main_function_name: Option<String>,
}

impl ControlFlowParser {
    pub fn new(input: &str) -> Self {
        let lines: Vec<String> = input.lines().map(|s| s.trim().to_string()).collect();
        eprintln!("[PARSER] Loaded {} lines", lines.len());
        for (i, line) in lines.iter().enumerate() {
            eprintln!("[PARSER] Line {}: {:?}", i, line);
        }
        ControlFlowParser { lines, pos: 0, has_main: false, just_run: false, main_function_name: None }
    }

    pub fn parse(&mut self) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();

        while self.pos < self.lines.len() {
            let line = match self.current_line() {
                Some(l) if !l.is_empty() => l.clone(),
                _ => {
                    self.advance();
                    continue;
                }
            };
            
            // Check for preprocessor directives
            if line.starts_with("@main") {
                self.has_main = true;
                self.advance();
                // Lookahead to get the function name
                if let Some(next_line) = self.current_line() {
                    let upper = next_line.to_uppercase();
                    if upper.starts_with("FUNCTION ") {
                        let after_fn = next_line.trim()[8..].trim();
                        if let Some(paren_pos) = after_fn.find('(') {
                            let func_name = after_fn[..paren_pos].trim().to_string();
                            self.main_function_name = Some(func_name);
                        } else {
                            let parts: Vec<&str> = after_fn.split_whitespace().collect();
                            if !parts.is_empty() {
                                self.main_function_name = Some(parts[0].to_string());
                            }
                        }
                    }
                }
                continue;
            }
            if line.starts_with("@justrunit") {
                self.just_run = true;
                self.advance();
                continue;
            }
            
            if let Some(stmt) = self.parse_statement()? {
                statements.push(stmt);
            }
        }

        Ok(statements)
    }

    fn current_line(&self) -> Option<&String> {
        self.lines.get(self.pos)
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn strip_inline_comment(line: &str) -> String {
        let mut chars = line.chars().peekable();
        let mut result = String::new();

        while let Some(ch) = chars.next() {
            if ch == '"' || ch == '\'' {
                result.push(ch);
                while let Some(next_ch) = chars.next() {
                    result.push(next_ch);
                    if next_ch == ch {
                        break;
                    }
                }
                continue;
            }

            if ch == '#' {
                break;
            }

            if ch == '/' {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '/' {
                        break;
                    }
                }
            }

            result.push(ch);
        }

        result
    }

    fn parse_expression(&self, expr_str: &str) -> Result<Expression, String> {
        let trimmed = expr_str.trim();
        
        // Parenthesized expression
        if trimmed.starts_with('(') && trimmed.ends_with(')') {
            let inner = &trimmed[1..trimmed.len()-1];
            // Check if it's a binary operation inside
            return self.parse_binary_op(inner);
        }
        
        // String literal
        if trimmed.starts_with('"') && trimmed.ends_with('"') {
            let content = &trimmed[1..trimmed.len()-1];
            return Ok(Expression::Literal(Value::String(content.to_string())));
        }
        if trimmed.starts_with('\'') && trimmed.ends_with('\'') {
            let content = &trimmed[1..trimmed.len()-1];
            return Ok(Expression::Literal(Value::String(content.to_string())));
        }
        
        // Number
        if let Ok(n) = trimmed.parse::<i64>() {
            return Ok(Expression::Literal(Value::Number(n)));
        }
        if let Ok(f) = trimmed.parse::<f64>() {
            return Ok(Expression::Literal(Value::Float(f)));
        }
        
        // Variable
        if trimmed.starts_with('$') {
            return Ok(Expression::Variable(trimmed[1..].to_string()));
        }
        
        // Binary operations (without parentheses)
        if trimmed.contains('+') || trimmed.contains('-') || trimmed.contains('*') || trimmed.contains('/') {
            return self.parse_binary_op(trimmed);
        }
        
        // C call
        if trimmed.starts_with("c::") {
            let rest = &trimmed[3..];
            if let Some(open_paren) = rest.find('(') {
                if rest.ends_with(')') {
                    let func_name = rest[..open_paren].trim();
                    let args_str = &rest[open_paren+1..rest.len()-1];
                    let args: Vec<Expression> = if args_str.trim().is_empty() {
                        Vec::new()
                    } else {
                        args_str.split(',').map(|s| self.parse_expression(s.trim())).collect::<Result<_, _>>()?
                    };
                    return Ok(Expression::CCall(func_name.to_string(), args));
                }
            }
        }
        
        // std:: call (Rust standard library functions)
        if trimmed.starts_with("std::") {
            let rest = &trimmed[5..];
            if let Some(open_paren) = rest.find('(') {
                if rest.ends_with(')') {
                    let func_name = rest[..open_paren].trim();
                    let args_str = &rest[open_paren+1..rest.len()-1];
                    let args: Vec<Expression> = if args_str.trim().is_empty() {
                        Vec::new()
                    } else {
                        args_str.split(',').map(|s| self.parse_expression(s.trim())).collect::<Result<_, _>>()?
                    };
                    return Ok(Expression::FunctionCall(format!("std::{}", func_name), args));
                }
            }
        }
        
        // Function call
        if let Some(open_paren) = trimmed.find('(') {
            if trimmed.ends_with(')') {
                let func_name = trimmed[..open_paren].trim();
                if !func_name.is_empty() {
                    let args_str = &trimmed[open_paren+1..trimmed.len()-1];
                    let args: Vec<Expression> = if args_str.trim().is_empty() {
                        Vec::new()
                    } else {
                        args_str.split(',').map(|s| self.parse_expression(s.trim())).collect::<Result<_, _>>()?
                    };
                    return Ok(Expression::FunctionCall(func_name.to_string(), args));
                }
            }
        }
        
        // Default to string literal for now
        Ok(Expression::Literal(Value::String(trimmed.to_string())))
    }

    fn parse_binary_op(&self, expr_str: &str) -> Result<Expression, String> {
        let trimmed = expr_str.trim();
        
        // Find the operator with lowest precedence (rightmost)
        // Precedence: +, - (lower) then *, / (higher)
        let mut add_sub_pos = None;
        let mut mul_div_pos = None;
        let mut paren_depth = 0;
        
        for (i, ch) in trimmed.chars().enumerate() {
            match ch {
                '(' => paren_depth += 1,
                ')' => paren_depth -= 1,
                '+' | '-' if paren_depth == 0 => add_sub_pos = Some(i),
                '*' | '/' if paren_depth == 0 => {
                    if mul_div_pos.is_none() {
                        mul_div_pos = Some(i);
                    }
                }
                _ => {}
            }
        }
        
        // Use addition/subtraction as primary split, or multiplication/division
        if let Some(pos) = add_sub_pos {
            let op_char = trimmed.chars().nth(pos).unwrap();
            let left = self.parse_expression(trimmed[..pos].trim())?;
            let right = self.parse_expression(trimmed[pos+1..].trim())?;
            let op = if op_char == '+' { BinOp::Add } else { BinOp::Sub };
            return Ok(Expression::BinaryOp(Box::new(left), op, Box::new(right)));
        } else if let Some(pos) = mul_div_pos {
            let op_char = trimmed.chars().nth(pos).unwrap();
            let left = self.parse_expression(trimmed[..pos].trim())?;
            let right = self.parse_expression(trimmed[pos+1..].trim())?;
            let op = if op_char == '*' { BinOp::Mul } else { BinOp::Div };
            return Ok(Expression::BinaryOp(Box::new(left), op, Box::new(right)));
        }
        
        // Not a binary op, parse as normal
        self.parse_simple_expression(expr_str)
    }

    fn parse_simple_expression(&self, expr_str: &str) -> Result<Expression, String> {
        let trimmed = expr_str.trim();
        
        // String literal
        if trimmed.starts_with('"') && trimmed.ends_with('"') {
            let content = &trimmed[1..trimmed.len()-1];
            return Ok(Expression::Literal(Value::String(content.to_string())));
        }
        if trimmed.starts_with('\'') && trimmed.ends_with('\'') {
            let content = &trimmed[1..trimmed.len()-1];
            return Ok(Expression::Literal(Value::String(content.to_string())));
        }
        
        // Number
        if let Ok(n) = trimmed.parse::<i64>() {
            return Ok(Expression::Literal(Value::Number(n)));
        }
        if let Ok(f) = trimmed.parse::<f64>() {
            return Ok(Expression::Literal(Value::Float(f)));
        }
        
        // Variable
        if trimmed.starts_with('$') {
            return Ok(Expression::Variable(trimmed[1..].to_string()));
        }
        
        // Default to string literal for now
        Ok(Expression::Literal(Value::String(trimmed.to_string())))
    }

    fn parse_expression_statement(&mut self, line: &str) -> Result<Option<Expression>, String> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        // Simple check: if it looks like a function call or expression
        if trimmed.contains('(') || trimmed.starts_with('$') || trimmed.starts_with('"') || trimmed.chars().next().unwrap().is_digit(10) {
            match self.parse_expression(trimmed) {
                Ok(expr) => Ok(Some(expr)),
                Err(_) => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    fn parse_statement(&mut self) -> Result<Option<Statement>, String> {
        let line = match self.current_line() {
            Some(l) if !l.is_empty() => Self::strip_inline_comment(l),
            _ => {
                self.advance();
                return Ok(None);
            }
        };

        let trimmed = line.trim();
        let upper = line.to_uppercase();

        if trimmed.starts_with("//") || upper.starts_with("#") {
            self.advance();
            return Ok(None);
        }

        if upper.starts_with("IF ") {
            self.parse_if()
        } else if upper.starts_with("WHILE ") {
            self.parse_while()
        } else if upper.starts_with("FOREACH ") {
            self.parse_foreach()
        } else if upper.starts_with("FOR ") {
            self.parse_for()
        } else if upper.starts_with("LOOP") {
            self.parse_loop()
        } else if upper.starts_with("BREAK") {
            self.parse_break()
        } else if upper.starts_with("CONTINUE") {
            self.parse_continue()
        } else if upper.starts_with("RETURN") {
            self.parse_return()
        } else if upper.starts_with("TRY") {
            self.parse_try()
        } else if upper.starts_with("STRUCT ") {
            self.parse_struct()
        } else if upper.starts_with("ENUM ") {
            self.parse_enum()
        } else if upper.starts_with("SCAN") {
            self.parse_scan()
        } else if upper.starts_with("SWITCH") {
            self.parse_switch()
        } else if upper.starts_with("CHAIN") {
            self.parse_chain()
        } else if upper.starts_with("FUNCTION ") {
            self.parse_function()
        } else if upper.starts_with("LET ") {
            self.parse_let()
        } else if let Some(stmt) = self.parse_assignment(&line)? {
            self.advance();
            Ok(Some(stmt))
        } else if let Some(expr) = self.parse_expression_statement(&line)? {
            self.advance();
            Ok(Some(Statement::Expression(expr)))
        } else if upper.starts_with("@") {
            self.advance();
            Ok(None)
        } else {
            self.advance();
            Ok(Some(Statement::Command(line)))
        }
    }

    fn parse_assignment(&mut self, line: &str) -> Result<Option<Statement>, String> {
        let trimmed = line.trim();
        if !trimmed.starts_with('$') {
            return Ok(None);
        }

        let mut chars = trimmed.chars();
        chars.next();

        let mut name = String::new();
        while let Some(ch) = chars.next() {
            if ch.is_alphanumeric() || ch == '_' || ch == '.' {
                name.push(ch);
            } else {
                break;
            }
        }

        if name.is_empty() {
            return Ok(None);
        }

        let rest: String = chars.collect();
        let rest = rest.trim_start();
        if !rest.starts_with('=') {
            return Ok(None);
        }

        let rhs = rest[1..].trim();
        if rhs.is_empty() {
            return Err("Assignment syntax: $var = expression".to_string());
        }

        let value = if rhs.starts_with('{') && rhs.ends_with('}') && rhs.len() > 2 {
            let block_content = &rhs[1..rhs.len()-1];
            let block_lines: Vec<String> = block_content.lines().map(|s| s.trim().to_string()).collect();
            let mut block_parser = ControlFlowParser::new(&block_lines.join("\n"));
            let statements = block_parser.parse()?;
            Expression::Block(statements)
        } else {
            self.parse_expression(rhs)?
        };

        self.advance();
        Ok(Some(Statement::Let { name, value }))
    }

    fn parse_function(&mut self) -> Result<Option<Statement>, String> {
        let line = self.current_line().unwrap().clone();
        self.advance();

        let trimmed = line.trim();
        let line_upper = trimmed.to_uppercase();
        let mut remainder = if line_upper.starts_with("FUNCTION ") {
            trimmed[8..].trim().to_string()
        } else {
            return Err("Expected 'function' keyword".to_string());
        };

        remainder = Self::strip_inline_comment(&remainder).trim_end().to_string();

        let mut brace_style = false;
        if remainder.ends_with('{') {
            brace_style = true;
            remainder = remainder[..remainder.len() - 1].trim_end().to_string();
        }

        let (name, params) = if let Some(open_paren) = remainder.find('(') {
            let close_paren = remainder.rfind(')').ok_or("Invalid function parameter list")?;
            let name = remainder[..open_paren].trim().to_string();
            let params_text = remainder[open_paren + 1..close_paren].trim();
            let param_list: Vec<String> = if params_text.is_empty() {
                Vec::new()
            } else {
                params_text
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            };
            (name, param_list)
        } else {
            let parts: Vec<&str> = remainder.split_whitespace().collect();
            if parts.is_empty() {
                return Err("FUNCTION syntax: FUNCTION name [arg1 arg2 ...]".to_string());
            }
            let name = parts[0].to_string();
            let params = parts[1..].iter().map(|s| s.to_string()).collect();
            (name, params)
        };

        let body = if brace_style {
            self.parse_block_until_matching_brace()?
        } else {
            let body = self.parse_block_until(&["ENDFUNCTION"])?;
            if let Some(line) = self.current_line() {
                let upper_line = line.to_uppercase();
                if upper_line.starts_with("ENDFUNCTION") {
                    self.advance();
                } else {
                    return Err("Expected ENDFUNCTION".to_string());
                }
            } else {
                return Err("Expected ENDFUNCTION".to_string());
            }
            body
        };

        Ok(Some(Statement::FunctionDef { name, params, body }))
    }

    fn parse_let(&mut self) -> Result<Option<Statement>, String> {
        let raw_line = self.current_line().unwrap().clone();
        let line = Self::strip_inline_comment(&raw_line);
        self.advance();

        let remainder = line[3..].trim();
        let parts: Vec<&str> = remainder.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err("LET syntax: LET name = value".to_string());
        }

        let name = parts[0].trim().to_string();
        if name.starts_with('$') {
            return Err("LET syntax: variable name must not start with '$'".to_string());
        }
        if name.is_empty() {
            return Err("LET syntax: missing variable name".to_string());
        }

        let rhs = parts[1].trim();
        let value = if rhs.starts_with('{') && rhs.ends_with('}') && rhs.len() > 2 {
            // Block expression
            let block_content = &rhs[1..rhs.len()-1];
            let block_lines: Vec<String> = block_content.lines().map(|s| s.trim().to_string()).collect();
            let mut block_parser = ControlFlowParser::new(&block_lines.join("\n"));
            let statements = block_parser.parse()?;
            Expression::Block(statements)
        } else {
            self.parse_expression(rhs)?
        };

        Ok(Some(Statement::Let { name, value }))
    }

    fn parse_struct(&mut self) -> Result<Option<Statement>, String> {
        let line = self.current_line().unwrap().clone();
        self.advance();

        let remainder = line[6..].trim();
        let mut header = remainder;
        let mut brace_style = remainder.ends_with('{');
        if brace_style {
            header = remainder.trim_end_matches('{').trim();
        }

        let name = header.to_string();
        let mut variants = Vec::new();
        let mut brace_depth: usize = if brace_style { 1 } else { 0 };

        while let Some(current_line) = self.current_line() {
            let trimmed = current_line.trim();
            if trimmed.is_empty() {
                self.advance();
                continue;
            }

            if !brace_style && trimmed == "{" {
                brace_style = true;
                brace_depth = 1;
                self.advance();
                continue;
            }

            if brace_style {
                if trimmed.contains('{') {
                    brace_depth += trimmed.matches('{').count();
                }
                if trimmed.contains('}') {
                    brace_depth = brace_depth.saturating_sub(trimmed.matches('}').count());
                    self.advance();
                    if brace_depth == 0 {
                        break;
                    }
                    continue;
                }

                if let Some(colon_pos) = trimmed.find(':') {
                    let field_name = trimmed[..colon_pos].trim().to_string();
                    if !field_name.is_empty() {
                        variants.push(field_name);
                    }
                }
                self.advance();
                continue;
            }

            break;
        }

        Ok(Some(Statement::StructDef { name, fields: variants }))
    }

    fn parse_enum(&mut self) -> Result<Option<Statement>, String> {
        let line = self.current_line().unwrap().clone();
        self.advance();

        let remainder = line[4..].trim();
        let mut header = remainder;
        let mut brace_style = remainder.ends_with('{');
        if brace_style {
            header = remainder.trim_end_matches('{').trim();
        }

        let name = header.to_string();
        let mut variants = Vec::new();
        let mut brace_depth: usize = if brace_style { 1 } else { 0 };

        while let Some(current_line) = self.current_line() {
            let trimmed = current_line.trim();
            if trimmed.is_empty() {
                self.advance();
                continue;
            }

            if !brace_style && trimmed == "{" {
                brace_style = true;
                brace_depth = 1;
                self.advance();
                continue;
            }

            if brace_style {
                if trimmed.contains('{') {
                    brace_depth += trimmed.matches('{').count();
                }
                if trimmed.contains('}') {
                    brace_depth = brace_depth.saturating_sub(trimmed.matches('}').count());
                    self.advance();
                    if brace_depth == 0 {
                        break;
                    }
                    continue;
                }

                let variant = trimmed.trim_end_matches(',').trim().to_string();
                if !variant.is_empty() {
                    variants.push(variant);
                }
                self.advance();
                continue;
            }

            break;
        }

        Ok(Some(Statement::EnumDef { name, variants }))
    }

    fn parse_scan(&mut self) -> Result<Option<Statement>, String> {
        let line = self.current_line().unwrap().clone();
        self.advance();

        let trimmed = line.trim();
        let keyword_len = 4;
        let remainder = trimmed[keyword_len..].trim();
        let mut expr = None;
        let enum_type: String;

        if remainder.to_lowercase().starts_with("of ") {
            enum_type = remainder[3..].trim().trim_end_matches('{').trim().to_string();
        } else if let Some(of_pos) = remainder.to_lowercase().find(" of ") {
            expr = Some(self.parse_expression(&remainder[..of_pos].trim())?);
            enum_type = remainder[of_pos + 4..].trim().trim_end_matches('{').trim().to_string();
        } else {
            return Err("SCAN syntax: SCAN [expr] OF EnumType {".to_string());
        }

        let mut branches: Vec<(String, Vec<Statement>)> = Vec::new();
        let mut brace_depth = 0;
        let mut in_body = remainder.ends_with('{');

        while let Some(current_line) = self.current_line() {
            let trimmed = current_line.trim();
            if trimmed.is_empty() {
                self.advance();
                continue;
            }

            if !in_body {
                if trimmed == "{" {
                    in_body = true;
                    brace_depth = 1;
                    self.advance();
                    continue;
                }
                return Err("Expected '{' after SCAN header".to_string());
            }

            if trimmed.contains('{') {
                brace_depth += trimmed.matches('{').count();
            }
            if trimmed.contains('}') {
                brace_depth = brace_depth.saturating_sub(trimmed.matches('}').count());
                self.advance();
                if brace_depth == 0 {
                    break;
                }
                continue;
            }

            if let Some(colon_pos) = trimmed.find(':') {
                let label = trimmed[..colon_pos].trim().to_string();
                let body_text = trimmed[colon_pos + 1..].trim();
                let body_stmt = if body_text.is_empty() {
                    Vec::new()
                } else {
                    let mut parser = ControlFlowParser::new(body_text);
                    if let Some(stmt) = parser.parse()?.into_iter().next() {
                        vec![stmt]
                    } else {
                        Vec::new()
                    }
                };
                branches.push((label, body_stmt));
                self.advance();
                continue;
            }

            self.advance();
        }

        Ok(Some(Statement::Scan { expr, enum_type, branches }))
    }

    fn parse_switch(&mut self) -> Result<Option<Statement>, String> {
        let line = self.current_line().unwrap().clone();
        self.advance();

        let trimmed = line.trim();
        if !trimmed.to_lowercase().starts_with("switch on ") {
            return Err("SWITCH syntax: SWITCH ON expr {".to_string());
        }

        let remainder = trimmed[9..].trim();
        let expr_str = remainder.trim_end_matches('{').trim();
        let expr = self.parse_expression(expr_str)?;

        let mut branches: Vec<(String, Vec<Statement>)> = Vec::new();
        let mut default_branch: Option<Vec<Statement>> = None;
        let mut brace_depth = 0;
        let mut in_body = trimmed.ends_with('{');

        while let Some(current_line) = self.current_line() {
            let trimmed = current_line.trim();
            if trimmed.is_empty() {
                self.advance();
                continue;
            }

            if !in_body {
                if trimmed == "{" {
                    in_body = true;
                    brace_depth = 1;
                    self.advance();
                    continue;
                }
                return Err("Expected '{' after SWITCH header".to_string());
            }

            if trimmed.contains('{') {
                brace_depth += trimmed.matches('{').count();
            }
            if trimmed.contains('}') {
                brace_depth = brace_depth.saturating_sub(trimmed.matches('}').count());
                self.advance();
                if brace_depth == 0 {
                    break;
                }
                continue;
            }

            if let Some(colon_pos) = trimmed.find(':') {
                let label = trimmed[..colon_pos].trim().to_string();
                let body_text = trimmed[colon_pos + 1..].trim();
                let body_stmt = if body_text.is_empty() {
                    Vec::new()
                } else {
                    let mut parser = ControlFlowParser::new(body_text);
                    if let Some(stmt) = parser.parse()?.into_iter().next() {
                        vec![stmt]
                    } else {
                        Vec::new()
                    }
                };

                if label.to_lowercase() == "default" {
                    default_branch = Some(body_stmt);
                } else {
                    branches.push((label, body_stmt));
                }
                self.advance();
                continue;
            }

            self.advance();
        }

        Ok(Some(Statement::Switch { expr, branches, default_branch }))
    }

    fn parse_foreach(&mut self) -> Result<Option<Statement>, String> {
        let line = self.current_line().unwrap().clone();
        self.advance();

        let foreach_line = line[7..].trim();
        let foreach_line = foreach_line.trim_end_matches(':').trim();
        let upper = foreach_line.to_uppercase();
        let in_pos = upper.find(" IN ").ok_or("FOREACH syntax: FOREACH var IN iterable".to_string())?;

        let var = foreach_line[..in_pos].trim().to_string();
        let iterable_str = foreach_line[in_pos + 4..].trim().trim_end_matches('{').trim();
        let iterable = self.parse_expression(iterable_str)?;

        let body = if line.trim_end().ends_with('{') {
            self.parse_block_until_matching_brace()?
        } else {
            let body = self.parse_block_until(&["ENDFOREACH"])?;
            self.advance();
            body
        };

        Ok(Some(Statement::Foreach { var, iterable, body }))
    }

    fn parse_return(&mut self) -> Result<Option<Statement>, String> {
        let line = self.current_line().unwrap().clone();
        self.advance();

        let remainder = line[6..].trim();
        let value = if remainder.is_empty() {
            None
        } else {
            Some(self.parse_expression(remainder)?)
        };

        Ok(Some(Statement::Return { value }))
    }

    fn parse_chain(&mut self) -> Result<Option<Statement>, String> {
        let line = self.current_line().unwrap().clone();
        self.advance();

        let trimmed = line.trim();
        let remainder = trimmed[5..].trim();

        let steps_str = if remainder.is_empty() {
            if let Some(next_line) = self.current_line() {
                if next_line.trim() == "{" {
                    self.advance();
                    self.collect_chain_block_lines_until_matching_brace()?
                } else {
                    return Err("Expected '{' after CHAIN".to_string());
                }
            } else {
                return Err("Expected '{' after CHAIN".to_string());
            }
        } else if remainder == "{" {
            self.collect_chain_block_lines_until_matching_brace()?
        } else if remainder.starts_with('{') {
            let after_brace = remainder[1..].trim();
            if after_brace.ends_with('}') {
                let body = after_brace[..after_brace.len() - 1].trim();
                body.lines()
                    .map(|line| line.trim().to_string())
                    .filter(|line| !line.is_empty())
                    .collect()
            } else {
                let mut lines = Vec::new();
                if !after_brace.is_empty() {
                    lines.push(after_brace.to_string());
                }
                let mut more_lines = self.collect_chain_block_lines_until_matching_brace()?;
                lines.append(&mut more_lines);
                lines
            }
        } else {
            self.collect_chain_lines_until_endchain()?
        };

        let steps: Vec<Expression> = steps_str.into_iter().map(|s| self.parse_expression(&s)).collect::<Result<_, _>>()?;

        Ok(Some(Statement::Chain { steps }))
    }

    fn collect_chain_block_lines_until_matching_brace(&mut self) -> Result<Vec<String>, String> {
        let mut steps = Vec::new();

        while let Some(line) = self.current_line() {
            let trimmed = line.trim();
            if trimmed == "}" {
                self.advance();
                return Ok(steps);
            }

            if trimmed.starts_with("}") {
                let remainder = trimmed[1..].trim();
                if !remainder.is_empty() {
                    self.lines[self.pos] = remainder.to_string();
                } else {
                    self.advance();
                }
                return Ok(steps);
            }

            if !trimmed.is_empty() {
                steps.push(line.clone());
            }
            self.advance();
        }

        Err("Expected matching '}' for CHAIN block".to_string())
    }

    fn collect_chain_lines_until_endchain(&mut self) -> Result<Vec<String>, String> {
        let mut steps = Vec::new();

        while let Some(line) = self.current_line() {
            let trimmed = line.trim();
            if trimmed.to_uppercase() == "ENDCHAIN" {
                self.advance();
                return Ok(steps);
            }

            if !trimmed.is_empty() {
                steps.push(line.clone());
            }
            self.advance();
        }

        Err("Expected ENDCHAIN to terminate CHAIN block".to_string())
    }

    fn parse_try(&mut self) -> Result<Option<Statement>, String> {
        let line = self.current_line().unwrap().clone();
        self.advance();

        let trimmed = line.trim();
        let try_block = if trimmed.ends_with('{') {
            self.parse_block_until_matching_brace()?
        } else {
            self.parse_block_until(&["CATCH", "ENDTRY"])?
        };

        let mut catch_block = None;

        if let Some(line) = self.current_line() {
            let trimmed_line = line.trim();
            let upper_line = trimmed_line.to_uppercase();
            if upper_line.starts_with("CATCH") {
                let catch_line = self.current_line().unwrap().clone();
                self.advance();
                let catch_trimmed = catch_line.trim();
                if catch_trimmed.ends_with('{') {
                    catch_block = Some(self.parse_block_until_matching_brace()?);
                } else {
                    let catch_body = self.parse_block_until(&["ENDTRY"])?;
                    catch_block = Some(catch_body);
                }
            }
        }

        if let Some(line) = self.current_line() {
            let trimmed_line = line.trim();
            let upper_line = trimmed_line.to_uppercase();
            if upper_line.starts_with("ENDTRY") {
                self.advance();
            } else if catch_block.is_none() {
                return Err("Expected ENDTRY".to_string());
            }
        } else if catch_block.is_none() {
            return Err("Expected ENDTRY".to_string());
        }

        Ok(Some(Statement::Try {
            try_block,
            catch_block,
        }))
    }

    fn parse_if(&mut self) -> Result<Option<Statement>, String> {
        let line = self.current_line().unwrap().clone();
        self.advance();

        let mut cond_str = line[3..].trim();
        let brace_style = cond_str.ends_with('{');
        if brace_style {
            cond_str = cond_str.trim_end_matches('{').trim();
        }
        cond_str = cond_str.trim_end_matches(':').trim();

        let condition = self.parse_condition(cond_str)?;
        let then_block = if brace_style {
            self.parse_block_until_matching_brace()?
        } else {
            self.parse_block_until(&["ELIF", "ELSE", "ENDIF"])?
        };

        let mut elif_blocks = Vec::new();
        let mut else_block = None;

        loop {
            let line_opt = self.current_line();
            if line_opt.is_none() {
                if !brace_style {
                    return Err("Expected ENDIF".to_string());
                }
                break;
            }

            let line = line_opt.unwrap().clone();
            let upper = line.to_uppercase();

            if brace_style {
                let mut trimmed = line.trim();
                if trimmed.starts_with('}') {
                    trimmed = trimmed[1..].trim_start();
                }
                if trimmed.starts_with("else if") || trimmed.starts_with("ELSE IF") {
                    let remainder = if trimmed.starts_with("else if") {
                        &trimmed[7..]
                    } else {
                        &trimmed[8..]
                    };
                    self.advance();
                    let mut elif_cond_str = remainder.trim();
                    if elif_cond_str.ends_with('{') {
                        elif_cond_str = elif_cond_str.trim_end_matches('{').trim();
                    }
                    let elif_cond = self.parse_condition(elif_cond_str)?;
                    let elif_body = self.parse_block_until_matching_brace()?;
                    elif_blocks.push((elif_cond, elif_body));
                } else if trimmed.starts_with("else") {
                    self.advance();
                    let rest = trimmed[4..].trim();
                    if rest == "{" || (rest.is_empty() && self.current_line().map(|l| l.trim()) == Some("{")) {
                        if rest.is_empty() && self.current_line().map(|l| l.trim()) == Some("{") {
                            self.advance();
                        }
                        else_block = Some(self.parse_block_until_matching_brace()?);
                    } else {
                        return Err("Expected '{' after else".to_string());
                    }
                    break;
                } else {
                    break;
                }
            } else {
                if upper.starts_with("ELIF ") {
                    let cond_line = self.current_line().unwrap().clone();
                    self.advance();
                    let mut cond_str = cond_line[5..].trim();
                    let elif_brace_style = cond_str.ends_with('{');
                    if elif_brace_style {
                        cond_str = cond_str.trim_end_matches('{').trim();
                    }
                    let cond_str = cond_str.trim_end_matches(':').trim();
                    let elif_cond = self.parse_condition(cond_str)?;
                    let elif_body = if elif_brace_style {
                        self.parse_block_until_matching_brace()?
                    } else {
                        self.parse_block_until(&["ELIF", "ELSE", "ENDIF"])?
                    };
                    elif_blocks.push((elif_cond, elif_body));
                } else if upper.starts_with("ELSE") {
                    self.advance();
                    let else_line = self.current_line().unwrap_or(&line).clone();
                    let else_brace_style = else_line.trim().ends_with('{');
                    if else_brace_style {
                        self.advance();
                    }
                    else_block = Some(if else_brace_style {
                        self.parse_block_until_matching_brace()?
                    } else {
                        self.parse_block_until(&["ENDIF"])?
                    });
                    break;
                } else if upper.starts_with("ENDIF") {
                    self.advance();
                    break;
                } else {
                    return Err("Unexpected token in IF statement".to_string());
                }
            }
        }

        Ok(Some(Statement::If {
            condition,
            then_block,
            elif_blocks,
            else_block,
        }))
    }

    fn parse_while(&mut self) -> Result<Option<Statement>, String> {
        let line = self.current_line().unwrap().clone();

        let mut cond_str = line[6..].trim();
        let brace_style = cond_str.ends_with('{');
        if brace_style {
            cond_str = cond_str.trim_end_matches('{').trim();
        }
        let cond_str = cond_str.trim_end_matches(':').trim();

        let condition = self.parse_condition(cond_str)?;
        self.advance();  // Move past while line
        
        let body = if brace_style {
            // Brace was on same line, now parse the body
            self.parse_block_until_matching_brace()?
        } else {
            // Look for brace on next line
            if let Some(next_line) = self.current_line() {
                if next_line.trim() == "{" {
                    self.advance();  // Move past the {
                    self.parse_block_until_matching_brace()?
                } else {
                    return Err("Expected '{' after while condition".to_string());
                }
            } else {
                return Err("Expected '{' after while condition".to_string());
            }
        };

        Ok(Some(Statement::While { condition, body }))
    }

    fn parse_for(&mut self) -> Result<Option<Statement>, String> {
        let line = self.current_line().unwrap().clone();
        self.advance();

        let for_line = line[4..].trim();
        let for_line = for_line.trim_end_matches(':').trim();

        if for_line.starts_with('(') {
            let open_paren = for_line.find('(').ok_or("Invalid FOR syntax")?;
            let close_paren = for_line.rfind(')').ok_or("Invalid FOR syntax")?;
            let inner = for_line[open_paren + 1..close_paren].trim();
            let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
            if parts.len() != 3 {
                return Err("FOR syntax: for(init, condition, update)".to_string());
            }

            let init = if parts[0].is_empty() {
                None
            } else {
                Some(self.parse_expression(parts[0])?)
            };

            let condition = self.parse_condition(parts[1])?;
            let update = if parts[2].is_empty() {
                None
            } else {
                Some(self.parse_expression(parts[2])?)
            };

            let body = if for_line.ends_with('{') {
                self.parse_block_until_matching_brace()?
            } else {
                let body = self.parse_block_until(&["ENDFOR"])?;
                self.advance();
                body
            };

            Ok(Some(Statement::ForLoop {
                init,
                condition,
                update,
                body,
            }))
        } else {
            let for_line = for_line.trim_end_matches('{').trim();
            let in_pos = for_line.to_uppercase().find(" IN ").ok_or("FOR syntax: FOR var IN item1 item2 ...".to_string())?;
            let var = for_line[..in_pos].trim().to_string();
            let items_text = for_line[in_pos + 4..].trim();
            let items: Vec<Expression> = items_text.split_whitespace().map(|s| self.parse_expression(s)).collect::<Result<_, _>>()?;

            let body = if line.trim_end().ends_with('{') {
                self.parse_block_until_matching_brace()?
            } else {
                let body = self.parse_block_until(&["ENDFOR"])?;
                self.advance();
                body
            };

            Ok(Some(Statement::For { var, items, body }))
        }
    }

    fn parse_loop(&mut self) -> Result<Option<Statement>, String> {
        let line = self.current_line().unwrap().clone();
        self.advance();

        let trimmed_line = line[5..].trim();
        let brace_style = trimmed_line.ends_with('{');
        let spec = if brace_style {
            trimmed_line.trim_end_matches('{').trim()
        } else {
            trimmed_line
        };

        let parts: Vec<&str> = spec.split_whitespace().collect();

        let mut count = None;
        let mut interval = None;

        if parts.len() >= 2 {
            match parts[0] {
                "count" => {
                    count = parts[1].parse().ok();
                }
                "interval" => {
                    interval = parts[1].parse().ok();
                }
                _ => return Err(format!("Invalid loop specifier: {}", parts[0])),
            }
        }

        let body = if brace_style {
            self.parse_block_until_matching_brace()?
        } else {
            let body = self.parse_block_until(&["ENDLOOP"])?;
            self.advance();
            body
        };

        Ok(Some(Statement::Loop {
            count,
            interval,
            body,
        }))
    }

    fn parse_break(&mut self) -> Result<Option<Statement>, String> {
        let line = self.current_line().unwrap().clone();
        self.advance();

        let break_line = line[5..].trim();

        let condition = if break_line.is_empty() {
            None
        } else {
            Some(self.parse_condition(break_line)?)
        };

        Ok(Some(Statement::Break { condition }))
    }

    fn parse_continue(&mut self) -> Result<Option<Statement>, String> {
        self.advance();
        Ok(Some(Statement::Continue))
    }

    fn parse_block_until(&mut self, end_keywords: &[&str]) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();

        loop {
            let line = match self.current_line() {
                Some(l) => l.to_uppercase(),
                None => return Err(format!("Expected one of: {}", end_keywords.join(", "))),
            };

            if end_keywords.iter().any(|kw| line.starts_with(kw)) {
                break;
            }

            if let Some(stmt) = self.parse_statement()? {
                statements.push(stmt);
            }
        }

        Ok(statements)
    }

    fn parse_block_until_matching_brace(&mut self) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();
        eprintln!("[BLOCK] Starting parse_block at pos={}, total lines={}", self.pos, self.lines.len());

        while let Some(line) = self.current_line() {
            let trimmed = line.trim();
            eprintln!("[BLOCK] loop: pos={} line={:?} current_line returns Some", self.pos, trimmed);
            
            // Skip empty lines
            if trimmed.is_empty() {
                self.advance();
                continue;
            }
            
            // If this line is just a closing brace, we're done
            if trimmed == "}" {
                eprintln!("[BLOCK] Found closing brace, returning {} statements", statements.len());
                self.advance();
                return Ok(statements);
            }
            
            // If line starts with }, we're done
            if trimmed.starts_with('}') {
                eprintln!("[BLOCK] Found line starting with close brace, returning {} statements", statements.len());
                self.advance();
                return Ok(statements);
            }

            // Parse the statement
            eprintln!("[BLOCK] Calling parse_statement at pos={}", self.pos);
            if let Some(stmt) = self.parse_statement()? {
                eprintln!("[BLOCK] Got statement, now at pos={}", self.pos);
                statements.push(stmt);
            } else {
                eprintln!("[BLOCK] parse_statement returned None, now at pos={}", self.pos);
            }
        }

        eprintln!("[BLOCK] Loop exited, pos={}, lines.len()={}, current_line returned None", self.pos, self.lines.len());
        Err("Expected matching }".to_string())
    }

    fn parse_condition(&mut self, cond_str: &str) -> Result<Condition, String> {
        fn find_top_level(cond: &str, pat: &str) -> Option<usize> {
            let mut depth = 0;
            let mut in_quote: Option<char> = None;
            let bytes = cond.as_bytes();
            let mut i = 0;

            while i < bytes.len() {
                let ch = bytes[i] as char;
                if let Some(q) = in_quote {
                    if ch == q {
                        in_quote = None;
                    }
                    i += 1;
                    continue;
                }

                if ch == '"' || ch == '\'' {
                    in_quote = Some(ch);
                    i += 1;
                    continue;
                }

                if ch == '(' {
                    depth += 1;
                } else if ch == ')' {
                    if depth > 0 {
                        depth -= 1;
                    }
                }

                if depth == 0 && cond[i..].starts_with(pat) {
                    return Some(i);
                }

                i += 1;
            }
            None
        }

        let mut cond = cond_str.trim();
        while cond.starts_with('(') && cond.ends_with(')') {
            let mut depth = 0;
            let mut matched = false;
            for (idx, ch) in cond.chars().enumerate() {
                if ch == '(' {
                    depth += 1;
                } else if ch == ')' {
                    depth -= 1;
                    if depth == 0 {
                        matched = idx == cond.len() - 1;
                        break;
                    }
                }
            }
            if matched {
                cond = cond[1..cond.len() - 1].trim();
            } else {
                break;
            }
        }

        if let Some(and_pos) = find_top_level(cond, " AND ").or_else(|| find_top_level(cond, "&&")) {
            let left = self.parse_condition(&cond[..and_pos])?;
            let right = if cond[and_pos..].starts_with("&&") {
                self.parse_condition(&cond[and_pos + 2..])?
            } else {
                self.parse_condition(&cond[and_pos + 5..])?
            };
            return Ok(Condition::And(Box::new(left), Box::new(right)));
        }

        if let Some(or_pos) = find_top_level(cond, " OR ").or_else(|| find_top_level(cond, "||")) {
            let left = self.parse_condition(&cond[..or_pos])?;
            let right = if cond[or_pos..].starts_with("||") {
                self.parse_condition(&cond[or_pos + 2..])?
            } else {
                self.parse_condition(&cond[or_pos + 4..])?
            };
            return Ok(Condition::Or(Box::new(left), Box::new(right)));
        }

        if let Some(is_not_pos) = cond.to_uppercase().find(" IS NOT ") {
            let left = self.parse_expression(&cond[..is_not_pos].trim())?;
            let right = self.parse_expression(&cond[is_not_pos + 8..].trim())?;
            return Ok(Condition::IsNot(left, right));
        }

        if let Some(is_pos) = cond.to_uppercase().find(" IS ") {
            let left = self.parse_expression(&cond[..is_pos].trim())?;
            let right = self.parse_expression(&cond[is_pos + 4..].trim())?;
            return Ok(Condition::Is(left, right));
        }

        for (op_str, op) in &[
            ("==", CompareOp::Eq),
            ("!=", CompareOp::Ne),
            ("<=", CompareOp::Le),
            (">=", CompareOp::Ge),
            ("<", CompareOp::Lt),
            (">", CompareOp::Gt),
        ] {
            if let Some(pos) = cond.find(op_str) {
                let left = self.parse_expression(&cond[..pos].trim())?;
                let right = self.parse_expression(&cond[pos + op_str.len()..].trim())?;
                return Ok(Condition::Compare(left, op.clone(), right));
            }
        }

        Ok(Condition::Command(self.parse_expression(cond)?))
    }
}

#[derive(Clone)]
pub struct Environment {
    pub variables: HashMap<String, Value>,
    pub functions: HashMap<String, FunctionDef>,
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            variables: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }

    pub fn get_path(&self, path: &str) -> Option<Value> {
        let mut parts = path.split('.');
        let first = parts.next()?;
        let mut current = self.variables.get(first)?.clone();

        for part in parts {
            match current {
                Value::Struct(_, ref fields) => {
                    if let Some(next) = fields.get(part) {
                        current = next.clone();
                    } else {
                        return None;
                    }
                }
                _ => return None,
            }
        }

        Some(current)
    }

    pub fn get_path_ref(&self, path: &str) -> Option<&Value> {
        let mut parts = path.split('.');
        let first = parts.next()?;
        let mut current = self.variables.get(first)?;

        for part in parts {
            match current {
                Value::Struct(_, ref fields) => {
                    current = fields.get(part)?;
                }
                _ => return None,
            }
        }

        Some(current)
    }

    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.variables.values()
    }

    pub fn set(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }

    pub fn remove(&mut self, name: &str) {
        self.variables.remove(name);
    }

    pub fn set_function(&mut self, name: String, function: FunctionDef) {
        self.functions.insert(name, function);
    }

    pub fn get_function(&self, name: &str) -> Option<&FunctionDef> {
        self.functions.get(name)
    }

    pub fn expand_vars(&self, s: &str) -> String {
        let mut result = String::with_capacity(s.len() + 16);
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' {
                let mut var_name = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' || ch == '.' {
                        var_name.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                if let Some(value) = self.get_path_ref(&var_name) {
                    match value {
                        Value::String(s) => result.push_str(s),
                        Value::Number(n) => result.push_str(&n.to_string()),
                        Value::Float(f) => result.push_str(&f.to_string()),
                        Value::Bool(b) => result.push_str(if *b { "true" } else { "false" }),
                        Value::List(items) => result.push_str(&items.join(" ")),
                        Value::Struct(type_name, fields) => {
                            let mut parts: Vec<String> = Vec::new();
                            parts.reserve(fields.len());
                            for (key, value) in fields {
                                parts.push(format!("{}: {}", key, value.as_string()));
                            }
                            result.push_str(&format!("{} {{ {} }}", type_name, parts.join(" ")));
                        }
                        Value::Enum(type_name, variant) => result.push_str(&format!("{}.{}", type_name, variant)),
                    }
                } else {
                    result.push('$');
                    result.push_str(&var_name);
                }
            } else {
                result.push(c);
            }
        }

        result
    }
}
