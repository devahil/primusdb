/*
 * PrimusDB GraphQL Service — Parser
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.0.0
 */

//! Recursive-descent parser for the GraphQL subset supported by PrimusDB.
//!
//! Parses queries and mutations with fields, aliases, arguments, variables,
//! lists, input objects and the standard scalar literals. The GraphQL
//! constructs outside this subset (fragments, directives, interfaces,
//! subscriptions, unions, enums-as-types) produce an explicit
//! [`ParseError::NotSupported`] instead of a confusing generic failure.

use super::ast::{
    Argument, Document, Field, Operation, OperationType, TypeRef, Value, VariableDefinition,
};
use std::collections::BTreeMap;

/// A GraphQL parsing error.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// Human-readable description.
    pub message: String,
    /// 0-based token offset in the source.
    pub position: usize,
}

impl ParseError {
    fn new(message: impl Into<String>, position: usize) -> Self {
        Self {
            message: message.into(),
            position,
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (at offset {})", self.message, self.position)
    }
}

/// The kind of a single token.
#[derive(Debug, Clone, PartialEq)]
enum TokKind {
    /// A name / enum / type token (`[A-Za-z_][A-Za-z0-9_]*`).
    Name(String),
    /// A signed integer literal.
    Int(i64),
    /// A float literal.
    Float(f64),
    /// A string literal (already unescaped).
    Str(String),
    /// `!`
    Bang,
    /// `$`
    Dollar,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `...`
    Spread,
    /// `:`
    Colon,
    /// `=`
    Eq,
    /// `@`
    At,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `|`
    Pipe,
}

#[derive(Debug, Clone, PartialEq)]
struct Token {
    kind: TokKind,
    pos: usize,
}

/// A small lexer producing the token stream for a GraphQL document.
struct Lexer {
    src: Vec<char>,
    pos: usize,
}

impl Lexer {
    fn new(input: &str) -> Self {
        Self {
            src: input.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.src.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() || c == ',' => {
                    self.pos += 1;
                }
                Some('#') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }
    }

    fn lex(mut self) -> Result<Vec<Token>, ParseError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_trivia();
            let pos = self.pos;
            let c = match self.peek() {
                None => break,
                Some(c) => c,
            };
            let kind = match c {
                '!' => {
                    self.pos += 1;
                    TokKind::Bang
                }
                '$' => {
                    self.pos += 1;
                    TokKind::Dollar
                }
                '(' => {
                    self.pos += 1;
                    TokKind::LParen
                }
                ')' => {
                    self.pos += 1;
                    TokKind::RParen
                }
                ':' => {
                    self.pos += 1;
                    TokKind::Colon
                }
                '=' => {
                    self.pos += 1;
                    TokKind::Eq
                }
                '@' => {
                    self.pos += 1;
                    TokKind::At
                }
                '[' => {
                    self.pos += 1;
                    TokKind::LBracket
                }
                ']' => {
                    self.pos += 1;
                    TokKind::RBracket
                }
                '{' => {
                    self.pos += 1;
                    TokKind::LBrace
                }
                '}' => {
                    self.pos += 1;
                    TokKind::RBrace
                }
                '|' => {
                    self.pos += 1;
                    TokKind::Pipe
                }
                '.' => {
                    if self.src.get(self.pos..self.pos + 3) == Some(&['.', '.', '.']) {
                        self.pos += 3;
                        TokKind::Spread
                    } else {
                        return Err(ParseError::new("unexpected '.'", pos));
                    }
                }
                '"' => {
                    let s = self.lex_string(pos)?;
                    TokKind::Str(s)
                }
                '-' | '0'..='9' => {
                    let (int, float) = self.lex_number(pos)?;
                    match float {
                        Some(f) => TokKind::Float(f),
                        None => TokKind::Int(int),
                    }
                }
                c if c.is_alphabetic() || c == '_' => {
                    let name = self.lex_name();
                    TokKind::Name(name)
                }
                _ => {
                    return Err(ParseError::new(format!("unexpected character '{c}'"), pos));
                }
            };
            tokens.push(Token { kind, pos });
        }
        Ok(tokens)
    }

    fn lex_name(&mut self) -> String {
        let mut out = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                out.push(c);
                self.pos += 1;
            } else {
                break;
            }
        }
        out
    }

    fn lex_string(&mut self, start: usize) -> Result<String, ParseError> {
        self.pos += 1; // opening quote
        let mut out = String::new();
        loop {
            let c = match self.bump() {
                Some(c) => c,
                None => return Err(ParseError::new("unterminated string", start)),
            };
            match c {
                '"' => break,
                '\\' => match self.bump() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('/') => out.push('/'),
                    Some('b') => out.push('\u{0008}'),
                    Some('f') => out.push('\u{000C}'),
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some('u') => {
                        let mut hex = String::new();
                        for _ in 0..4 {
                            match self.bump() {
                                Some(h) => hex.push(h),
                                None => {
                                    return Err(ParseError::new("unterminated \\u escape", start))
                                }
                            }
                        }
                        let code = u32::from_str_radix(&hex, 16).map_err(|_| {
                            ParseError::new(format!("invalid \\u escape '\\u{hex}'"), start)
                        })?;
                        out.push(
                            char::from_u32(code)
                                .ok_or_else(|| ParseError::new("invalid unicode escape", start))?,
                        );
                    }
                    Some(other) => {
                        return Err(ParseError::new(
                            format!("invalid escape '\\{other}'"),
                            start,
                        ))
                    }
                    None => return Err(ParseError::new("unterminated string", start)),
                },
                other => out.push(other),
            }
        }
        Ok(out)
    }

    fn lex_number(&mut self, start: usize) -> Result<(i64, Option<f64>), ParseError> {
        let mut text = String::new();
        if self.peek() == Some('-') {
            text.push('-');
            self.pos += 1;
        }
        while matches!(self.peek(), Some('0'..='9')) {
            text.push(self.bump().unwrap());
        }
        let mut is_float = false;
        if self.peek() == Some('.') {
            is_float = true;
            text.push(self.bump().unwrap());
            while matches!(self.peek(), Some('0'..='9')) {
                text.push(self.bump().unwrap());
            }
        }
        if matches!(self.peek(), Some('e') | Some('E')) {
            is_float = true;
            text.push(self.bump().unwrap());
            if matches!(self.peek(), Some('+') | Some('-')) {
                text.push(self.bump().unwrap());
            }
            while matches!(self.peek(), Some('0'..='9')) {
                text.push(self.bump().unwrap());
            }
        }
        if text == "-" || text.is_empty() {
            return Err(ParseError::new("invalid number literal", start));
        }
        if is_float {
            let f: f64 = text
                .parse()
                .map_err(|_| ParseError::new(format!("invalid float literal '{text}'"), start))?;
            Ok((0, Some(f)))
        } else {
            let i: i64 = text
                .parse()
                .map_err(|_| ParseError::new(format!("invalid int literal '{text}'"), start))?;
            Ok((i, None))
        }
    }
}

/// A recursive-descent parser over a token stream.
pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    /// Parse a complete GraphQL document.
    pub fn parse_document(input: &str) -> Result<Document, ParseError> {
        let tokens = Lexer::new(input).lex()?;
        let mut parser = Parser { tokens, cursor: 0 };
        let mut operations = Vec::new();
        while !parser.at_end() {
            operations.push(parser.parse_operation()?);
        }
        if operations.is_empty() {
            return Err(ParseError::new("empty document", 0));
        }
        Ok(Document { operations })
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }

    fn at_end(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    fn bump(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.cursor).cloned();
        if t.is_some() {
            self.cursor += 1;
        }
        t
    }

    fn expect(&mut self, kind: &TokKind, what: &str) -> Result<Token, ParseError> {
        match self.bump() {
            Some(t) if std::mem::discriminant(&t.kind) == std::mem::discriminant(kind) => Ok(t),
            Some(t) => Err(ParseError::new(
                format!("expected {what}, found {}", describe(&t.kind)),
                t.pos,
            )),
            None => Err(ParseError::new(
                format!("expected {what}, found end of input"),
                0,
            )),
        }
    }

    fn parse_operation(&mut self) -> Result<Operation, ParseError> {
        let (operation_type, name) = match self.peek() {
            Some(t) if t.kind == TokKind::Name("query".to_string()) => {
                self.bump();
                (OperationType::Query, self.try_parse_operation_name())
            }
            Some(t) if t.kind == TokKind::Name("mutation".to_string()) => {
                self.bump();
                (OperationType::Mutation, self.try_parse_operation_name())
            }
            Some(t) if t.kind == TokKind::Name("subscription".to_string()) => {
                let pos = t.pos;
                return Err(ParseError::new("subscriptions are not supported", pos));
            }
            Some(t) if t.kind == TokKind::Name("fragment".to_string()) => {
                let pos = t.pos;
                return Err(ParseError::new("fragments are not supported", pos));
            }
            // Anonymous shorthand: `{ ... }`.
            Some(t) if t.kind == TokKind::LBrace => (OperationType::Query, None),
            _ => {
                let t = self.peek().cloned().unwrap_or_else(|| Token {
                    kind: TokKind::Name("".to_string()),
                    pos: 0,
                });
                return Err(ParseError::new(
                    format!("expected operation, found {}", describe(&t.kind)),
                    t.pos,
                ));
            }
        };

        let variable_definitions = if self
            .peek()
            .map(|t| t.kind == TokKind::LParen)
            .unwrap_or(false)
        {
            self.parse_variable_definitions()?
        } else {
            Vec::new()
        };

        let selection_set = self.parse_selection_set()?;
        Ok(Operation {
            operation_type,
            name,
            variable_definitions,
            selection_set,
        })
    }

    fn try_parse_operation_name(&mut self) -> Option<String> {
        match self.peek() {
            Some(t) if matches!(t.kind, TokKind::Name(_)) => {
                let name = match self.bump().unwrap().kind {
                    TokKind::Name(n) => n,
                    _ => unreachable!(),
                };
                Some(name)
            }
            _ => None,
        }
    }

    fn parse_variable_definitions(&mut self) -> Result<Vec<VariableDefinition>, ParseError> {
        self.expect(&TokKind::LParen, "'('")?;
        let mut defs = Vec::new();
        while !self.at_end()
            && self
                .peek()
                .map(|t| t.kind != TokKind::RParen)
                .unwrap_or(false)
        {
            self.expect(&TokKind::Dollar, "'$'")?;
            let name = match self.bump() {
                Some(Token {
                    kind: TokKind::Name(n),
                    ..
                }) => n,
                Some(t) => {
                    return Err(ParseError::new(
                        format!("expected variable name, found {}", describe(&t.kind)),
                        t.pos,
                    ))
                }
                None => {
                    return Err(ParseError::new(
                        "expected variable name, found end of input",
                        0,
                    ))
                }
            };
            self.expect(&TokKind::Colon, "':'")?;
            let var_type = self.parse_type()?;
            let default = if self.peek().map(|t| t.kind == TokKind::Eq).unwrap_or(false) {
                self.bump();
                Some(self.parse_value()?)
            } else {
                None
            };
            defs.push(VariableDefinition {
                name,
                var_type,
                default,
            });
        }
        self.expect(&TokKind::RParen, "')'")?;
        Ok(defs)
    }

    fn parse_type(&mut self) -> Result<TypeRef, ParseError> {
        let base = match self.peek() {
            Some(Token {
                kind: TokKind::Name(n),
                ..
            }) => {
                let n = n.clone();
                self.bump();
                TypeRef::Named(n)
            }
            Some(t) if t.kind == TokKind::LBracket => {
                self.bump();
                let inner = self.parse_type()?;
                self.expect(&TokKind::RBracket, "']'")?;
                TypeRef::List(Box::new(inner))
            }
            Some(t) => {
                return Err(ParseError::new(
                    format!("expected type, found {}", describe(&t.kind)),
                    t.pos,
                ))
            }
            None => return Err(ParseError::new("expected type, found end of input", 0)),
        };
        if self
            .peek()
            .map(|t| t.kind == TokKind::Bang)
            .unwrap_or(false)
        {
            self.bump();
            Ok(TypeRef::NonNull(Box::new(base)))
        } else {
            Ok(base)
        }
    }

    fn parse_selection_set(&mut self) -> Result<Vec<Field>, ParseError> {
        self.expect(&TokKind::LBrace, "'{'")?;
        let mut fields = Vec::new();
        while !self.at_end()
            && self
                .peek()
                .map(|t| t.kind != TokKind::RBrace)
                .unwrap_or(false)
        {
            if self
                .peek()
                .map(|t| t.kind == TokKind::Spread)
                .unwrap_or(false)
            {
                return Err(ParseError::new("fragments are not supported", 0));
            }
            if self.peek().map(|t| t.kind == TokKind::At).unwrap_or(false) {
                return Err(ParseError::new("directives are not supported", 0));
            }
            fields.push(self.parse_field()?);
        }
        self.expect(&TokKind::RBrace, "'}'")?;
        Ok(fields)
    }

    fn parse_field(&mut self) -> Result<Field, ParseError> {
        let first = match self.bump() {
            Some(t) => t,
            None => {
                return Err(ParseError::new(
                    "expected field name, found end of input",
                    0,
                ))
            }
        };
        let first_name = match first.kind {
            TokKind::Name(n) => n,
            other => {
                return Err(ParseError::new(
                    format!("expected field name, found {}", describe(&other)),
                    first.pos,
                ))
            }
        };

        // Alias detection: `name : name ...`
        let (alias, name) = if self
            .peek()
            .map(|t| t.kind == TokKind::Colon)
            .unwrap_or(false)
        {
            let colon_pos = self.peek().unwrap().pos;
            self.bump();
            match self.bump() {
                Some(Token {
                    kind: TokKind::Name(n),
                    ..
                }) => (Some(first_name), n),
                Some(t) => {
                    return Err(ParseError::new(
                        format!(
                            "expected field name after alias, found {}",
                            describe(&t.kind)
                        ),
                        colon_pos,
                    ))
                }
                None => {
                    return Err(ParseError::new(
                        "expected field name after alias, found end of input",
                        colon_pos,
                    ))
                }
            }
        } else {
            (None, first_name)
        };

        let arguments = if self
            .peek()
            .map(|t| t.kind == TokKind::LParen)
            .unwrap_or(false)
        {
            self.parse_arguments()?
        } else {
            Vec::new()
        };

        let selection_set = if self
            .peek()
            .map(|t| t.kind == TokKind::LBrace)
            .unwrap_or(false)
        {
            self.parse_selection_set()?
        } else {
            Vec::new()
        };

        Ok(Field {
            alias,
            name,
            arguments,
            selection_set,
        })
    }

    fn parse_arguments(&mut self) -> Result<Vec<Argument>, ParseError> {
        self.expect(&TokKind::LParen, "'('")?;
        let mut args = Vec::new();
        while !self.at_end()
            && self
                .peek()
                .map(|t| t.kind != TokKind::RParen)
                .unwrap_or(false)
        {
            let name = match self.bump() {
                Some(Token {
                    kind: TokKind::Name(n),
                    ..
                }) => n,
                Some(t) => {
                    return Err(ParseError::new(
                        format!("expected argument name, found {}", describe(&t.kind)),
                        t.pos,
                    ))
                }
                None => {
                    return Err(ParseError::new(
                        "expected argument name, found end of input",
                        0,
                    ))
                }
            };
            self.expect(&TokKind::Colon, "':'")?;
            let value = self.parse_value()?;
            args.push(Argument { name, value });
        }
        self.expect(&TokKind::RParen, "')'")?;
        Ok(args)
    }

    fn parse_value(&mut self) -> Result<Value, ParseError> {
        let tok = match self.peek() {
            Some(t) => t.clone(),
            None => return Err(ParseError::new("expected value, found end of input", 0)),
        };
        match tok.kind {
            TokKind::Dollar => {
                self.bump();
                match self.bump() {
                    Some(Token {
                        kind: TokKind::Name(n),
                        ..
                    }) => Ok(Value::Variable(n)),
                    Some(t) => Err(ParseError::new(
                        format!("expected variable name, found {}", describe(&t.kind)),
                        t.pos,
                    )),
                    None => Err(ParseError::new(
                        "expected variable name, found end of input",
                        0,
                    )),
                }
            }
            TokKind::Int(i) => {
                self.bump();
                Ok(Value::Int(i))
            }
            TokKind::Float(f) => {
                self.bump();
                Ok(Value::Float(f))
            }
            TokKind::Str(s) => {
                self.bump();
                Ok(Value::String(s))
            }
            TokKind::Name(ref n) => {
                self.bump();
                match n.as_str() {
                    "true" => Ok(Value::Bool(true)),
                    "false" => Ok(Value::Bool(false)),
                    "null" => Ok(Value::Null),
                    other => Ok(Value::Enum(other.to_string())),
                }
            }
            TokKind::LBracket => {
                self.bump();
                let mut items = Vec::new();
                while !self.at_end()
                    && self
                        .peek()
                        .map(|t| t.kind != TokKind::RBracket)
                        .unwrap_or(false)
                {
                    items.push(self.parse_value()?);
                }
                self.expect(&TokKind::RBracket, "']'")?;
                Ok(Value::List(items))
            }
            TokKind::LBrace => {
                self.bump();
                let mut map = BTreeMap::new();
                while !self.at_end()
                    && self
                        .peek()
                        .map(|t| t.kind != TokKind::RBrace)
                        .unwrap_or(false)
                {
                    let key = match self.bump() {
                        Some(Token {
                            kind: TokKind::Name(n),
                            ..
                        }) => n,
                        Some(Token {
                            kind: TokKind::Str(s),
                            ..
                        }) => s,
                        Some(t) => {
                            return Err(ParseError::new(
                                format!("expected object key, found {}", describe(&t.kind)),
                                t.pos,
                            ))
                        }
                        None => {
                            return Err(ParseError::new(
                                "expected object key, found end of input",
                                0,
                            ))
                        }
                    };
                    self.expect(&TokKind::Colon, "':'")?;
                    let value = self.parse_value()?;
                    map.insert(key, value);
                }
                self.expect(&TokKind::RBrace, "'}'")?;
                Ok(Value::Object(map))
            }
            _ => Err(ParseError::new(
                format!("expected value, found {}", describe(&tok.kind)),
                tok.pos,
            )),
        }
    }
}

fn describe(kind: &TokKind) -> String {
    match kind {
        TokKind::Name(n) => format!("'{n}'"),
        TokKind::Int(i) => format!("{i}"),
        TokKind::Float(f) => format!("{f}"),
        TokKind::Str(_) => "string".to_string(),
        TokKind::Bang => "'!'".to_string(),
        TokKind::Dollar => "'$'".to_string(),
        TokKind::LParen => "'('".to_string(),
        TokKind::RParen => "')'".to_string(),
        TokKind::Spread => "'...'".to_string(),
        TokKind::Colon => "':'".to_string(),
        TokKind::Eq => "'='".to_string(),
        TokKind::At => "'@'".to_string(),
        TokKind::LBracket => "'['".to_string(),
        TokKind::RBracket => "']'".to_string(),
        TokKind::LBrace => "'{'".to_string(),
        TokKind::RBrace => "'}'".to_string(),
        TokKind::Pipe => "'|'".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_query() {
        let doc = Parser::parse_document(
            "{ table(storageType: \"Document\", name: \"users\") { name } }",
        )
        .unwrap();
        assert_eq!(doc.operations.len(), 1);
        let op = &doc.operations[0];
        assert_eq!(op.operation_type, OperationType::Query);
        assert_eq!(op.name, None);
        assert_eq!(op.selection_set.len(), 1);
        let field = &op.selection_set[0];
        assert_eq!(field.name, "table");
        assert_eq!(field.arguments.len(), 2);
        assert_eq!(field.selection_set[0].name, "name");
    }

    #[test]
    fn test_parse_named_operation_with_variables() {
        let doc = Parser::parse_document(
            "query GetUsers($limit: Int = 10) { tables(storageType: \"Document\") { records(limit: $limit) { id } } }",
        )
        .unwrap();
        let op = &doc.operations[0];
        assert_eq!(op.name.as_deref(), Some("GetUsers"));
        assert_eq!(op.variable_definitions.len(), 1);
        assert_eq!(op.variable_definitions[0].name, "limit");
        assert_eq!(op.variable_definitions[0].default, Some(Value::Int(10)));
    }

    #[test]
    fn test_parse_alias_and_literals() {
        let doc = Parser::parse_document(
            "{ a: search(query: \"rust\", limit: 5) { total hits { score } } }",
        )
        .unwrap();
        let field = &doc.operations[0].selection_set[0];
        assert_eq!(field.alias.as_deref(), Some("a"));
        assert_eq!(field.name, "search");
    }

    #[test]
    fn test_parse_object_and_list_values() {
        let doc = Parser::parse_document(
            "mutation { insert(storageType: \"Document\", table: \"t\", data: \"{\\\"a\\\":1}\") }",
        )
        .unwrap();
        assert_eq!(doc.operations[0].operation_type, OperationType::Mutation);
    }

    #[test]
    fn test_parse_errors_are_explicit() {
        let err = Parser::parse_document("fragment F on T { id }").unwrap_err();
        assert!(err.message.contains("fragments"), "{err:?}");

        let err = Parser::parse_document("subscription { x }").unwrap_err();
        assert!(err.message.contains("subscriptions"), "{err:?}");
    }

    #[test]
    fn test_lexer_comments_and_commas() {
        let doc = Parser::parse_document(
            "# leading comment\n{ x: search(query: \"a\", limit: 3) { total } } # trailing",
        )
        .unwrap();
        assert_eq!(
            doc.operations[0].selection_set[0].alias.as_deref(),
            Some("x")
        );
    }
}
