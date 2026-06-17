/*
 * PrimusDB Query Parser Module
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 2.0.0 - Rewritten with proper tokenizer + recursive descent parser
 */

use crate::query::QueryLanguage;
use crate::query::UqlQuery;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Select,
    Insert,
    Into,
    Values,
    Update,
    Set,
    Delete,
    From,
    Where,
    Join,
    Inner,
    Left,
    Right,
    Full,
    Outer,
    Cross,
    On,
    And,
    Or,
    Not,
    In,
    Is,
    Null,
    As,
    Order,
    By,
    Asc,
    Desc,
    Limit,
    Offset,
    Group,
    Having,
    Like,
    Between,
    Exists,
    All,
    Any,
    Some,
    Union,
    Intersect,
    Except,
    Distinct,
    Create,
    Table,
    Drop,
    Alter,
    Add,
    Column,
    Modify,
    Constraint,
    Primary,
    Key,
    Foreign,
    References,
    Index,
    Unique,
    Rename,
    Sequence,
    View,
    Trigger,
    Before,
    After,
    InsteadOf,
    Function,
    Execute,
    Raise,
    Cache,
    Cycle,
    MinValue,
    MaxValue,
    Start,
    With,
    OwnedBy,
    InformationSchema,
    True,
    False,
    Over,
    Partition,
    Recursive,
    Identifier(String),
    Number(String),
    String(String),
    Comma,
    Dot,
    Semicolon,
    Star,
    OpenParen,
    CloseParen,
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Plus,
    Minus,
    Slash,
    Percent,
    Assignment,
    Eof,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Identifier(s) => write!(f, "{}", s),
            Token::Number(s) => write!(f, "{}", s),
            Token::String(s) => write!(f, "'{}'", s),
            _ => write!(f, "{:?}", self),
        }
    }
}

#[derive(Debug)]
pub struct Tokenizer {
    input: Vec<char>,
    pos: usize,
    #[allow(dead_code, clippy::vec_box)]
    nested_queries: Vec<Box<ParsedQuery>>,
    #[allow(dead_code)]
    window_functions: Vec<WindowFunctionColumn>,
}

impl Tokenizer {
    pub fn new(input: &str) -> Self {
        Tokenizer {
            input: input.chars().collect(),
            pos: 0,
            nested_queries: Vec::new(),
            window_functions: Vec::new(),
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.input.get(self.pos).copied();
        self.pos += 1;
        ch
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_identifier(&mut self, start: char) -> String {
        let mut s = String::new();
        s.push(start);
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' || ch == '$' {
                s.push(self.advance().unwrap());
            } else {
                break;
            }
        }
        s
    }

    fn read_quoted_identifier(&mut self) -> String {
        let mut s = String::new();
        self.advance(); // skip opening "
        while let Some(ch) = self.peek() {
            match ch {
                '"' => {
                    self.advance();
                    break;
                }
                '\\' => {
                    self.advance();
                    if let Some(esc) = self.advance() {
                        s.push(esc);
                    }
                }
                _ => {
                    s.push(self.advance().unwrap());
                }
            }
        }
        s
    }

    fn read_string(&mut self) -> String {
        let mut s = String::new();
        self.advance(); // skip opening '
        while let Some(ch) = self.peek() {
            match ch {
                '\'' => {
                    self.advance();
                    if self.peek() == Some('\'') {
                        self.advance();
                        s.push('\'');
                    } else {
                        break;
                    }
                }
                _ => {
                    s.push(self.advance().unwrap());
                }
            }
        }
        s
    }

    fn read_number(&mut self, start: char) -> String {
        let mut s = String::new();
        s.push(start);
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() || ch == '.' || ch == 'e' || ch == 'E' {
                s.push(self.advance().unwrap());
            } else {
                break;
            }
        }
        s
    }

    fn keyword_or_identifier(s: &str) -> Token {
        match s.to_uppercase().as_str() {
            "SELECT" => Token::Select,
            "INSERT" => Token::Insert,
            "INTO" => Token::Into,
            "VALUES" => Token::Values,
            "UPDATE" => Token::Update,
            "SET" => Token::Set,
            "DELETE" => Token::Delete,
            "FROM" => Token::From,
            "WHERE" => Token::Where,
            "JOIN" => Token::Join,
            "INNER" => Token::Inner,
            "LEFT" => Token::Left,
            "RIGHT" => Token::Right,
            "FULL" => Token::Full,
            "OUTER" => Token::Outer,
            "CROSS" => Token::Cross,
            "ON" => Token::On,
            "AND" => Token::And,
            "OR" => Token::Or,
            "NOT" => Token::Not,
            "IN" => Token::In,
            "IS" => Token::Is,
            "NULL" => Token::Null,
            "AS" => Token::As,
            "ORDER" => Token::Order,
            "BY" => Token::By,
            "ASC" => Token::Asc,
            "DESC" => Token::Desc,
            "LIMIT" => Token::Limit,
            "OFFSET" => Token::Offset,
            "GROUP" => Token::Group,
            "HAVING" => Token::Having,
            "LIKE" => Token::Like,
            "BETWEEN" => Token::Between,
            "EXISTS" => Token::Exists,
            "ALL" => Token::All,
            "ANY" => Token::Any,
            "SOME" => Token::Some,
            "UNION" => Token::Union,
            "INTERSECT" => Token::Intersect,
            "EXCEPT" => Token::Except,
            "DISTINCT" => Token::Distinct,
            "CREATE" => Token::Create,
            "TABLE" => Token::Table,
            "DROP" => Token::Drop,
            "ALTER" => Token::Alter,
            "ADD" => Token::Add,
            "COLUMN" => Token::Column,
            "MODIFY" => Token::Modify,
            "CONSTRAINT" => Token::Constraint,
            "PRIMARY" => Token::Primary,
            "KEY" => Token::Key,
            "FOREIGN" => Token::Foreign,
            "REFERENCES" => Token::References,
            "INDEX" => Token::Index,
            "UNIQUE" => Token::Unique,
            "RENAME" => Token::Rename,
            "SEQUENCE" => Token::Sequence,
            "VIEW" => Token::View,
            "TRIGGER" => Token::Trigger,
            "BEFORE" => Token::Before,
            "AFTER" => Token::After,
            "INSTEAD" => Token::InsteadOf,
            "OF" => Token::InsteadOf,
            "FUNCTION" => Token::Function,
            "EXECUTE" => Token::Execute,
            "RAISE" => Token::Raise,
            "CACHE" => Token::Cache,
            "CYCLE" => Token::Cycle,
            "MINVALUE" => Token::MinValue,
            "MAXVALUE" => Token::MaxValue,
            "START" => Token::Start,
            "WITH" => Token::With,
            "OWNED" => Token::OwnedBy,
            "INFORMATION_SCHEMA" => Token::InformationSchema,
            "TRUE" => Token::True,
            "FALSE" => Token::False,
            "OVER" => Token::Over,
            "PARTITION" => Token::Partition,
            "RECURSIVE" => Token::Recursive,
            _ => Token::Identifier(s.to_string()),
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace();
            match self.peek() {
                None => {
                    tokens.push(Token::Eof);
                    break;
                }
                Some(ch) => match ch {
                    ',' => {
                        self.advance();
                        tokens.push(Token::Comma);
                    }
                    '.' => {
                        self.advance();
                        tokens.push(Token::Dot);
                    }
                    ';' => {
                        self.advance();
                        tokens.push(Token::Semicolon);
                    }
                    '*' => {
                        self.advance();
                        tokens.push(Token::Star);
                    }
                    '(' => {
                        self.advance();
                        tokens.push(Token::OpenParen);
                    }
                    ')' => {
                        self.advance();
                        tokens.push(Token::CloseParen);
                    }
                    '+' => {
                        self.advance();
                        tokens.push(Token::Plus);
                    }
                    '-' => {
                        self.advance();
                        tokens.push(Token::Minus);
                    }
                    '/' => {
                        self.advance();
                        tokens.push(Token::Slash);
                    }
                    '%' => {
                        self.advance();
                        tokens.push(Token::Percent);
                    }
                    '=' => {
                        self.advance();
                        tokens.push(Token::Eq);
                    }
                    '!' => {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            tokens.push(Token::Neq);
                        } else {
                            return Err(crate::Error::ValidationError(
                                "Expected '=' after '!'".to_string(),
                            ));
                        }
                    }
                    '>' => {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            tokens.push(Token::Gte);
                        } else {
                            tokens.push(Token::Gt);
                        }
                    }
                    '<' => {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            tokens.push(Token::Lte);
                        } else if self.peek() == Some('>') {
                            self.advance();
                            tokens.push(Token::Neq);
                        } else {
                            tokens.push(Token::Lt);
                        }
                    }
                    '\'' => {
                        let s = self.read_string();
                        tokens.push(Token::String(s));
                    }
                    '"' => {
                        let s = self.read_quoted_identifier();
                        tokens.push(Token::Identifier(s));
                    }
                    '0'..='9' => {
                        self.advance();
                        let n = self.read_number(ch);
                        tokens.push(Token::Number(n));
                    }
                    _ if ch.is_alphabetic() || ch == '_' => {
                        self.advance();
                        let ident = self.read_identifier(ch);
                        tokens.push(Self::keyword_or_identifier(&ident));
                    }
                    _ => {
                        return Err(crate::Error::ValidationError(format!(
                            "Unexpected character: '{}'",
                            ch
                        )));
                    }
                },
            }
        }

        Ok(tokens)
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    #[allow(clippy::vec_box)]
    nested_queries: Vec<Box<ParsedQuery>>,
    window_functions: Vec<WindowFunctionColumn>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            pos: 0,
            nested_queries: Vec::new(),
            window_functions: Vec::new(),
        }
    }

    fn peek(&self) -> Token {
        self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<Token> {
        let tok = self.peek();
        if std::mem::discriminant(&tok) == std::mem::discriminant(expected) || tok == *expected {
            Ok(self.advance())
        } else {
            Err(crate::Error::ValidationError(format!(
                "Expected {:?}, found {:?}",
                expected, tok
            )))
        }
    }

    #[allow(dead_code)]
    fn expect_any(&mut self, expected: &[Token]) -> Result<Token> {
        let tok = self.peek();
        for exp in expected {
            if std::mem::discriminant(&tok) == std::mem::discriminant(exp) || tok == *exp {
                return Ok(self.advance());
            }
        }
        Err(crate::Error::ValidationError(format!(
            "Expected one of {:?}, found {:?}",
            expected, tok
        )))
    }

    fn peek_is(&self, expected: &Token) -> bool {
        let tok = self.peek();
        tok == *expected
    }

    #[allow(dead_code)]
    fn peek_is_identifier(&self) -> bool {
        matches!(self.peek(), Token::Identifier(_))
    }

    fn peek_is_any(&self, expected: &[Token]) -> bool {
        expected.iter().any(|e| self.peek_is(e))
    }

    fn parse_comma_list<T>(&mut self, parser: fn(&mut Parser) -> Result<T>) -> Result<Vec<T>> {
        let mut items = Vec::new();
        items.push(parser(self)?);
        while self.peek_is(&Token::Comma) {
            self.advance();
            items.push(parser(self)?);
        }
        Ok(items)
    }

    pub fn parse_query(&mut self) -> Result<ParsedQuery> {
        let tok = self.peek();
        let mut result = if tok == Token::With || tok == Token::Recursive {
            let ctes = self.parse_cte_list()?;
            let mut stmt = self.parse_query()?;
            stmt.ctes = ctes;
            stmt
        } else {
            match tok {
                Token::Select => self.parse_select(),
                Token::Insert => self.parse_insert(),
                Token::Update => self.parse_update(),
                Token::Delete => self.parse_delete(),
                Token::Create => self.parse_create(),
                Token::Drop => self.parse_drop(),
                Token::Alter => self.parse_alter(),
                _ => Err(crate::Error::ValidationError(format!(
                    "Expected SQL statement, found {:?}",
                    tok
                ))),
            }?
        };

        if self.peek_is(&Token::Union)
            || self.peek_is(&Token::Intersect)
            || self.peek_is(&Token::Except)
        {
            result = self.parse_set_operations(result)?;
        }

        Ok(result)
    }

    fn parse_cte_list(&mut self) -> Result<Vec<CTEDefinition>> {
        self.expect(&Token::With)?;
        let _recursive = if self.peek_is(&Token::Recursive) {
            self.advance();
            true
        } else {
            false
        };
        let mut ctes = Vec::new();
        loop {
            let name = self.parse_identifier()?;
            let columns = if self.peek_is(&Token::OpenParen) {
                self.advance();
                let cols = self.parse_comma_list(Self::parse_identifier)?;
                self.expect(&Token::CloseParen)?;
                Some(cols)
            } else {
                None
            };
            self.expect(&Token::As)?;
            self.expect(&Token::OpenParen)?;
            let mut cte_body = self.parse_select()?;
            if self.peek_is(&Token::Union)
                || self.peek_is(&Token::Intersect)
                || self.peek_is(&Token::Except)
            {
                cte_body = self.parse_set_operations(cte_body)?;
            }
            self.expect(&Token::CloseParen)?;
            ctes.push(CTEDefinition {
                name,
                columns,
                query: Box::new(cte_body),
            });
            if self.peek_is(&Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        Ok(ctes)
    }

    fn parse_window_spec(&mut self) -> Result<WindowSpec> {
        self.expect(&Token::Over)?;
        self.expect(&Token::OpenParen)?;

        let mut partition_by = Vec::new();
        let mut order_by = Vec::new();
        let mut frame = None;

        if self.peek_is(&Token::Partition) {
            self.advance();
            self.expect(&Token::By)?;
            partition_by = self.parse_comma_list(Self::parse_identifier)?;
        }

        if self.peek_is(&Token::Order) {
            self.advance();
            self.expect(&Token::By)?;
            order_by = self.parse_comma_list(Self::parse_order_by)?;
        }

        if self.peek_is_any(&[
            Token::Identifier("ROWS".to_string()),
            Token::Identifier("RANGE".to_string()),
            Token::Identifier("GROUPS".to_string()),
        ]) {
            let tok = self.advance();
            let frame_type = if let Token::Identifier(s) = &tok {
                if s.eq_ignore_ascii_case("ROWS") {
                    WindowFrameType::Rows
                } else if s.eq_ignore_ascii_case("RANGE") {
                    WindowFrameType::Range
                } else {
                    WindowFrameType::Groups
                }
            } else {
                WindowFrameType::Rows
            };

            if self.peek_is(&Token::Between) {
                self.advance();
                let start = self.parse_frame_bound()?;
                self.expect(&Token::And)?;
                let end = self.parse_frame_bound()?;
                frame = Some(WindowFrame {
                    frame_type,
                    start,
                    end: Some(end),
                });
            } else {
                let start = self.parse_frame_bound()?;
                frame = Some(WindowFrame {
                    frame_type,
                    start,
                    end: None,
                });
            }
        }

        self.expect(&Token::CloseParen)?;

        Ok(WindowSpec {
            partition_by,
            order_by,
            frame,
        })
    }

    fn parse_frame_bound(&mut self) -> Result<FrameBound> {
        if self.peek_is(&Token::Identifier("UNBOUNDED".to_string())) {
            self.advance();
            if self.peek_is(&Token::Identifier("PRECEDING".to_string())) {
                self.advance();
                Ok(FrameBound::UnboundedPreceding)
            } else {
                self.advance(); // FOLLOWING
                Ok(FrameBound::UnboundedFollowing)
            }
        } else if self.peek_is(&Token::Identifier("CURRENT".to_string())) {
            self.advance();
            self.advance(); // ROW
            Ok(FrameBound::CurrentRow)
        } else if matches!(self.peek(), Token::Number(_)) {
            let n = if let Token::Number(s) = self.advance() {
                s.parse::<u64>().unwrap_or(0)
            } else {
                0
            };
            if self.peek_is(&Token::Identifier("PRECEDING".to_string())) {
                self.advance();
                Ok(FrameBound::NPreceding(n))
            } else {
                self.advance(); // FOLLOWING
                Ok(FrameBound::NFollowing(n))
            }
        } else {
            Ok(FrameBound::CurrentRow)
        }
    }

    fn parse_select(&mut self) -> Result<ParsedQuery> {
        self.expect(&Token::Select)?;

        let distinct = if self.peek_is(&Token::Distinct) {
            self.advance();
            true
        } else {
            false
        };

        let columns = if self.peek_is(&Token::Star) {
            self.advance();
            vec!["*".to_string()]
        } else {
            self.parse_comma_list(Self::parse_select_column)?
        };

        let mut source_tables = Vec::new();
        let mut joins = Vec::new();
        let mut conditions = None;
        let mut group_by = Vec::new();
        let mut having = None;
        let mut order_by = Vec::new();
        let mut limit = None;
        let mut offset = None;

        if self.peek_is(&Token::From) {
            self.advance();
            source_tables = self.parse_comma_list(Self::parse_table_ref)?;

            while self.peek_is(&Token::Join)
                || self.peek_is(&Token::Inner)
                || self.peek_is(&Token::Left)
                || self.peek_is(&Token::Right)
                || self.peek_is(&Token::Full)
                || self.peek_is(&Token::Cross)
            {
                joins.push(self.parse_join_clause()?);
            }
        }

        if self.peek_is(&Token::Where) {
            self.advance();
            conditions = Some(self.parse_expression()?);
        }

        if self.peek_is(&Token::Group) {
            self.advance();
            self.expect(&Token::By)?;
            group_by = self.parse_comma_list(Self::parse_identifier)?;

            if self.peek_is(&Token::Having) {
                self.advance();
                having = Some(self.parse_expression()?);
            }
        }

        if self.peek_is(&Token::Order) {
            self.advance();
            self.expect(&Token::By)?;
            order_by = self.parse_comma_list(Self::parse_order_by)?;
        }

        if self.peek_is(&Token::Limit) {
            self.advance();
            limit = Some(self.parse_number()?);
        }

        if self.peek_is(&Token::Offset) {
            self.advance();
            offset = Some(self.parse_number()?);
        }

        Ok(ParsedQuery {
            operation: QueryOperation::Select,
            source_tables,
            target_table: None,
            columns: columns.clone(),
            conditions,
            joins,
            order_by,
            group_by,
            aggregations: self.extract_aggregations(&columns),
            limit,
            offset,
            set_operations: vec![],
            nested_queries: std::mem::take(&mut self.nested_queries),
            distinct,
            having,
            ctes: vec![],
            window_functions: std::mem::take(&mut self.window_functions),
        })
    }

    fn parse_select_column(&mut self) -> Result<String> {
        let expr = self.parse_expression()?;
        if self.peek_is(&Token::As) {
            self.advance();
            let alias = self.parse_identifier()?;
            Ok(format!("{} AS {}", expr, alias))
        } else if self.peek_is(&Token::Identifier("".to_string())) {
            let next = self.peek();
            if let Token::Identifier(_) = &next {
                let alias = self.parse_identifier()?;
                Ok(format!("{} {}", expr, alias))
            } else {
                Ok(expr)
            }
        } else {
            Ok(expr)
        }
    }

    fn parse_table_ref(&mut self) -> Result<String> {
        let table = self.parse_identifier()?;
        if self.peek_is(&Token::As) {
            self.advance();
            self.parse_identifier()?; // consume alias
        } else if self.peek_is(&Token::Identifier("".to_string())) {
            self.parse_identifier()?; // consume alias
        }
        Ok(table)
    }

    fn parse_join_clause(&mut self) -> Result<JoinClause> {
        let join_type = if self.peek_is(&Token::Join) {
            self.advance();
            JoinType::Inner
        } else if self.peek_is(&Token::Inner) {
            self.advance();
            self.expect(&Token::Join)?;
            JoinType::Inner
        } else if self.peek_is(&Token::Left) {
            self.advance();
            if self.peek_is(&Token::Outer) {
                self.advance();
            }
            self.expect(&Token::Join)?;
            JoinType::Left
        } else if self.peek_is(&Token::Right) {
            self.advance();
            if self.peek_is(&Token::Outer) {
                self.advance();
            }
            self.expect(&Token::Join)?;
            JoinType::Right
        } else if self.peek_is(&Token::Full) {
            self.advance();
            if self.peek_is(&Token::Outer) {
                self.advance();
            }
            self.expect(&Token::Join)?;
            JoinType::Full
        } else if self.peek_is(&Token::Cross) {
            self.advance();
            self.expect(&Token::Join)?;
            JoinType::Cross
        } else {
            return Err(crate::Error::ValidationError(format!(
                "Expected JOIN type, found {:?}",
                self.peek()
            )));
        };

        let table = self.parse_identifier()?;

        if self.peek_is(&Token::As) {
            self.advance();
            self.parse_identifier()?;
        } else if self.peek_is(&Token::Identifier("".to_string())) {
            self.parse_identifier()?;
        }

        let mut condition = String::new();
        if self.peek_is(&Token::On) {
            self.advance();
            condition = self.parse_expression()?;
        }

        Ok(JoinClause {
            join_type,
            table,
            condition,
            engine_hint: None,
        })
    }

    fn parse_order_by(&mut self) -> Result<OrderByClause> {
        let column = self.parse_identifier()?;
        let direction = if self.peek_is(&Token::Asc) {
            self.advance();
            "ASC".to_string()
        } else if self.peek_is(&Token::Desc) {
            self.advance();
            "DESC".to_string()
        } else {
            "ASC".to_string()
        };
        Ok(OrderByClause { column, direction })
    }

    fn parse_insert(&mut self) -> Result<ParsedQuery> {
        self.expect(&Token::Insert)?;
        self.expect(&Token::Into)?;
        let target_table = Some(self.parse_identifier()?);

        let mut columns = Vec::new();
        if self.peek_is(&Token::OpenParen) {
            self.advance();
            columns = self.parse_comma_list(Self::parse_identifier)?;
            self.expect(&Token::CloseParen)?;
        }

        self.expect(&Token::Values)?;
        self.expect(&Token::OpenParen)?;
        let values = self.parse_comma_list(Self::parse_value)?;
        self.expect(&Token::CloseParen)?;

        Ok(ParsedQuery {
            operation: QueryOperation::Insert,
            source_tables: vec![],
            target_table,
            columns,
            conditions: Some(values.join(", ")),
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        })
    }

    fn parse_update(&mut self) -> Result<ParsedQuery> {
        self.expect(&Token::Update)?;
        let target_table = Some(self.parse_identifier()?);

        self.expect(&Token::Set)?;
        let mut set_clauses = Vec::new();
        loop {
            let col = self.parse_identifier()?;
            self.expect(&Token::Eq)?;
            let val = self.parse_value()?;
            set_clauses.push(format!("{} = {}", col, val));
            if self.peek_is(&Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        let conditions = if self.peek_is(&Token::Where) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(ParsedQuery {
            operation: QueryOperation::Update,
            source_tables: vec![],
            target_table,
            columns: set_clauses,
            conditions,
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        })
    }

    fn parse_delete(&mut self) -> Result<ParsedQuery> {
        self.expect(&Token::Delete)?;
        if self.peek_is(&Token::From) {
            self.advance();
        }
        let target_table = Some(self.parse_identifier()?);

        let conditions = if self.peek_is(&Token::Where) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(ParsedQuery {
            operation: QueryOperation::Delete,
            source_tables: vec![],
            target_table,
            columns: vec![],
            conditions,
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        })
    }

    fn parse_create(&mut self) -> Result<ParsedQuery> {
        self.expect(&Token::Create)?;
        match self.peek() {
            Token::Table => {
                self.advance();
                self.parse_create_table()
            }
            Token::Index => {
                self.advance();
                self.parse_create_index()
            }
            Token::Sequence => {
                self.advance();
                self.parse_create_sequence()
            }
            Token::View => {
                self.advance();
                self.parse_create_view()
            }
            Token::Trigger => {
                self.advance();
                self.parse_create_trigger()
            }
            _ => Err(crate::Error::ValidationError(format!(
                "Expected TABLE/INDEX/SEQUENCE/VIEW/TRIGGER after CREATE, found {:?}",
                self.peek()
            ))),
        }
    }

    fn parse_create_table(&mut self) -> Result<ParsedQuery> {
        let target_table = Some(self.parse_identifier()?);

        // Parse column definitions
        let mut columns = Vec::new();
        if self.peek_is(&Token::OpenParen) {
            self.advance();
            // Parse column definitions or just store raw
            let mut depth = 1;
            let mut def = String::new();
            while depth > 0 {
                match self.peek() {
                    Token::OpenParen => {
                        depth += 1;
                        def.push('(');
                        self.advance();
                    }
                    Token::CloseParen => {
                        depth -= 1;
                        if depth > 0 {
                            def.push(')');
                        }
                        self.advance();
                    }
                    Token::Eof => break,
                    _ => {
                        let tok = self.advance();
                        def.push_str(&format!("{} ", tok));
                    }
                }
            }
            columns.push(def.trim().to_string());
        }

        Ok(ParsedQuery {
            operation: QueryOperation::Create,
            source_tables: vec![],
            target_table,
            columns,
            conditions: None,
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        })
    }

    fn parse_create_index(&mut self) -> Result<ParsedQuery> {
        if self.peek_is(&Token::Unique) {
            self.advance();
        }
        let name = self.parse_identifier()?;
        self.expect(&Token::On)?;
        let table = self.parse_identifier()?;

        Ok(ParsedQuery {
            operation: QueryOperation::Create,
            source_tables: vec![],
            target_table: Some(table),
            columns: vec![name],
            conditions: None,
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        })
    }

    fn parse_create_sequence(&mut self) -> Result<ParsedQuery> {
        let name = self.parse_identifier()?;
        Ok(ParsedQuery {
            operation: QueryOperation::Create,
            source_tables: vec![],
            target_table: Some(name),
            columns: vec![],
            conditions: None,
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        })
    }

    fn parse_create_view(&mut self) -> Result<ParsedQuery> {
        let name = self.parse_identifier()?;
        self.expect(&Token::As)?;
        self.expect(&Token::Select)?;

        // Parse the inner SELECT as a nested query
        let inner = self.parse_select()?;

        Ok(ParsedQuery {
            operation: QueryOperation::Create,
            source_tables: vec![],
            target_table: Some(name),
            columns: vec![],
            conditions: None,
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![Box::new(inner)],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        })
    }

    fn parse_create_trigger(&mut self) -> Result<ParsedQuery> {
        let name = self.parse_identifier()?;

        let timing = if self.peek_is(&Token::Before) {
            self.advance();
            "BEFORE"
        } else if self.peek_is(&Token::After) {
            self.advance();
            "AFTER"
        } else if self.peek_is(&Token::InsteadOf) {
            self.advance();
            "INSTEAD OF"
        } else {
            return Err(crate::Error::ValidationError(
                "Expected BEFORE/AFTER/INSTEAD OF".to_string(),
            ));
        };

        let event = match self.peek() {
            Token::Insert => {
                self.advance();
                "INSERT"
            }
            Token::Update => {
                self.advance();
                "UPDATE"
            }
            Token::Delete => {
                self.advance();
                "DELETE"
            }
            _ => {
                return Err(crate::Error::ValidationError(
                    "Expected INSERT/UPDATE/DELETE".to_string(),
                ))
            }
        };

        self.expect(&Token::On)?;
        let table = self.parse_identifier()?;

        // Skip the body for now
        let mut body = String::new();
        if self.peek_is(&Token::Function) {
            self.advance();
            body = format!("FUNCTION {}", self.parse_identifier()?);
        }

        Ok(ParsedQuery {
            operation: QueryOperation::Create,
            source_tables: vec![],
            target_table: Some(table),
            columns: vec![format!("{} {} ON {} {}", timing, event, name, body)],
            conditions: None,
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        })
    }

    fn parse_drop(&mut self) -> Result<ParsedQuery> {
        self.expect(&Token::Drop)?;
        let operation = match self.peek() {
            Token::Table => {
                self.advance();
                QueryOperation::Drop
            }
            Token::Index => {
                self.advance();
                QueryOperation::Drop
            }
            Token::Sequence => {
                self.advance();
                QueryOperation::Drop
            }
            Token::View => {
                self.advance();
                QueryOperation::Drop
            }
            Token::Trigger => {
                self.advance();
                QueryOperation::Drop
            }
            _ => {
                return Err(crate::Error::ValidationError(format!(
                    "Expected TABLE/INDEX/SEQUENCE/VIEW/TRIGGER after DROP, found {:?}",
                    self.peek()
                )))
            }
        };
        let target_table = Some(self.parse_identifier()?);

        Ok(ParsedQuery {
            operation,
            source_tables: vec![],
            target_table,
            columns: vec![],
            conditions: None,
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        })
    }

    fn parse_alter(&mut self) -> Result<ParsedQuery> {
        self.expect(&Token::Alter)?;
        self.expect(&Token::Table)?;
        let target_table = Some(self.parse_identifier()?);

        match self.peek() {
            Token::Add => {
                self.advance();
                if self.peek_is(&Token::Constraint) {
                    self.advance();
                    let name = self.parse_identifier()?;
                    Ok(ParsedQuery {
                        operation: QueryOperation::Alter,
                        source_tables: vec![],
                        target_table,
                        columns: vec![],
                        conditions: Some(format!("ADD CONSTRAINT {}", name)),
                        joins: vec![],
                        order_by: vec![],
                        group_by: vec![],
                        aggregations: vec![],
                        limit: None,
                        offset: None,
                        set_operations: vec![],
                        nested_queries: vec![],
                        distinct: false,
                        having: None,
                        ctes: vec![],
                        window_functions: vec![],
                    })
                } else {
                    let col = self.parse_identifier()?;
                    Ok(ParsedQuery {
                        operation: QueryOperation::Alter,
                        source_tables: vec![],
                        target_table,
                        columns: vec![],
                        conditions: Some(format!("ADD COLUMN {}", col)),
                        joins: vec![],
                        order_by: vec![],
                        group_by: vec![],
                        aggregations: vec![],
                        limit: None,
                        offset: None,
                        set_operations: vec![],
                        nested_queries: vec![],
                        distinct: false,
                        having: None,
                        ctes: vec![],
                        window_functions: vec![],
                    })
                }
            }
            Token::Drop => {
                self.advance();
                if self.peek_is(&Token::Constraint) {
                    self.advance();
                    let name = self.parse_identifier()?;
                    Ok(ParsedQuery {
                        operation: QueryOperation::Alter,
                        source_tables: vec![],
                        target_table,
                        columns: vec![],
                        conditions: Some(format!("DROP CONSTRAINT {}", name)),
                        joins: vec![],
                        order_by: vec![],
                        group_by: vec![],
                        aggregations: vec![],
                        limit: None,
                        offset: None,
                        set_operations: vec![],
                        nested_queries: vec![],
                        distinct: false,
                        having: None,
                        ctes: vec![],
                        window_functions: vec![],
                    })
                } else {
                    let col = self.parse_identifier()?;
                    Ok(ParsedQuery {
                        operation: QueryOperation::Alter,
                        source_tables: vec![],
                        target_table,
                        columns: vec![],
                        conditions: Some(format!("DROP COLUMN {}", col)),
                        joins: vec![],
                        order_by: vec![],
                        group_by: vec![],
                        aggregations: vec![],
                        limit: None,
                        offset: None,
                        set_operations: vec![],
                        nested_queries: vec![],
                        distinct: false,
                        having: None,
                        ctes: vec![],
                        window_functions: vec![],
                    })
                }
            }
            Token::Modify => {
                self.advance();
                let col = self.parse_identifier()?;
                Ok(ParsedQuery {
                    operation: QueryOperation::Alter,
                    source_tables: vec![],
                    target_table,
                    columns: vec![],
                    conditions: Some(format!("MODIFY COLUMN {}", col)),
                    joins: vec![],
                    order_by: vec![],
                    group_by: vec![],
                    aggregations: vec![],
                    limit: None,
                    offset: None,
                    set_operations: vec![],
                    nested_queries: vec![],
                    distinct: false,
                    having: None,
                    ctes: vec![],
                    window_functions: vec![],
                })
            }
            Token::Rename => {
                self.advance();
                if self.peek_is(&Token::Identifier("to".to_string()))
                    || self.peek_is(&Token::Identifier("TO".to_string()))
                {
                    self.advance();
                }
                let new_name = self.parse_identifier()?;
                Ok(ParsedQuery {
                    operation: QueryOperation::Alter,
                    source_tables: vec![],
                    target_table,
                    columns: vec![new_name],
                    conditions: None,
                    joins: vec![],
                    order_by: vec![],
                    group_by: vec![],
                    aggregations: vec![],
                    limit: None,
                    offset: None,
                    set_operations: vec![],
                    nested_queries: vec![],
                    distinct: false,
                    having: None,
                    ctes: vec![],
                    window_functions: vec![],
                })
            }
            _ => Err(crate::Error::ValidationError(format!(
                "Expected ADD/DROP/MODIFY/RENAME after ALTER TABLE, found {:?}",
                self.peek()
            ))),
        }
    }

    fn parse_set_operations(&mut self, left: ParsedQuery) -> Result<ParsedQuery> {
        let mut operations = Vec::new();
        let mut current = left;

        while self.peek_is(&Token::Union)
            || self.peek_is(&Token::Intersect)
            || self.peek_is(&Token::Except)
        {
            let op_type = match self.peek() {
                Token::Union => {
                    self.advance();
                    if self.peek_is(&Token::All) {
                        self.advance();
                    }
                    SetOperationType::Union
                }
                Token::Intersect => {
                    self.advance();
                    SetOperationType::Intersect
                }
                Token::Except => {
                    self.advance();
                    SetOperationType::Except
                }
                _ => break,
            };

            let right = self.parse_select()?;
            operations.push(SetOperation {
                operation_type: op_type,
                query: Box::new(right),
            });
        }

        current.set_operations = operations;
        Ok(current)
    }

    fn parse_expression(&mut self) -> Result<String> {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> Result<String> {
        let mut left = self.parse_and_expr()?;
        while self.peek_is(&Token::Or) {
            self.advance();
            let right = self.parse_and_expr()?;
            left = format!("({} OR {})", left, right);
        }
        Ok(left)
    }

    fn parse_and_expr(&mut self) -> Result<String> {
        let mut left = self.parse_not_expr()?;
        while self.peek_is(&Token::And) {
            self.advance();
            let right = self.parse_not_expr()?;
            left = format!("({} AND {})", left, right);
        }
        Ok(left)
    }

    fn parse_not_expr(&mut self) -> Result<String> {
        if self.peek_is(&Token::Not) {
            self.advance();
            let expr = self.parse_not_expr()?;
            return Ok(format!("NOT {}", expr));
        }
        self.parse_predicate()
    }

    fn parse_predicate(&mut self) -> Result<String> {
        let lhs = self.parse_comparison()?;

        if self.peek_is(&Token::Is) {
            self.advance();
            if self.peek_is(&Token::Not) {
                self.advance();
                self.expect(&Token::Null)?;
                return Ok(format!("{} IS NOT NULL", lhs));
            } else {
                self.expect(&Token::Null)?;
                return Ok(format!("{} IS NULL", lhs));
            }
        }

        if self.peek_is(&Token::In) {
            self.advance();
            self.expect(&Token::OpenParen)?;
            // Check for subquery
            if self.peek_is(&Token::Select) {
                let subquery = self.parse_select()?;
                self.expect(&Token::CloseParen)?;
                let idx = self.nested_queries.len();
                self.nested_queries.push(Box::new(subquery));
                return Ok(format!("{} IN (SELECT __subq_{}__)", lhs, idx));
            }
            let mut items = Vec::new();
            loop {
                items.push(self.parse_value()?);
                if self.peek_is(&Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(&Token::CloseParen)?;
            return Ok(format!("{} IN ({})", lhs, items.join(", ")));
        }

        if self.peek_is(&Token::Like) {
            self.advance();
            let pattern = self.parse_value()?;
            return Ok(format!("{} LIKE {}", lhs, pattern));
        }

        if self.peek_is(&Token::Between) {
            self.advance();
            let low = self.parse_value()?;
            self.expect(&Token::And)?;
            let high = self.parse_value()?;
            return Ok(format!("{} BETWEEN {} AND {}", lhs, low, high));
        }

        if self.peek_is(&Token::Exists) {
            self.advance();
            self.expect(&Token::OpenParen)?;
            let subquery = self.parse_select()?;
            self.expect(&Token::CloseParen)?;
            let idx = self.nested_queries.len();
            self.nested_queries.push(Box::new(subquery));
            return Ok(format!("EXISTS (SELECT __subq_{}__)", idx));
        }

        Ok(lhs)
    }

    fn parse_comparison(&mut self) -> Result<String> {
        let left = self.parse_additive()?;

        if self.peek_is_any(&[
            Token::Eq,
            Token::Neq,
            Token::Gt,
            Token::Gte,
            Token::Lt,
            Token::Lte,
        ]) {
            let op = self.advance();
            let op_str = match op {
                Token::Eq => "=",
                Token::Neq => "!=",
                Token::Gt => ">",
                Token::Gte => ">=",
                Token::Lt => "<",
                Token::Lte => "<=",
                other => {
                    return Err(crate::Error::ValidationError(format!(
                        "Expected comparison operator, found {:?}",
                        other
                    )))
                }
            };
            let right = self.parse_additive()?;
            return Ok(format!("{} {} {}", left, op_str, right));
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<String> {
        let mut left = self.parse_multiplicative()?;
        while self.peek_is_any(&[Token::Plus, Token::Minus]) {
            let op = self.advance();
            let op_str = if op == Token::Plus { "+" } else { "-" };
            let right = self.parse_multiplicative()?;
            left = format!("({} {} {})", left, op_str, right);
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<String> {
        let mut left = self.parse_unary()?;
        while self.peek_is_any(&[Token::Star, Token::Slash, Token::Percent]) {
            let op = self.advance();
            let op_str = match op {
                Token::Star => "*",
                Token::Slash => "/",
                Token::Percent => "%",
                other => {
                    return Err(crate::Error::ValidationError(format!(
                        "Expected arithmetic operator, found {:?}",
                        other
                    )))
                }
            };
            let right = self.parse_unary()?;
            left = format!("({} {} {})", left, op_str, right);
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<String> {
        if self.peek_is(&Token::Minus) {
            self.advance();
            let expr = self.parse_primary()?;
            return Ok(format!("-{}", expr));
        }
        if self.peek_is(&Token::Plus) {
            self.advance();
            return self.parse_primary();
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<String> {
        match self.peek() {
            Token::Number(n) => {
                self.advance();
                Ok(n)
            }
            Token::String(s) => {
                self.advance();
                Ok(format!("'{}'", s))
            }
            Token::Null => {
                self.advance();
                Ok("NULL".to_string())
            }
            Token::True => {
                self.advance();
                Ok("TRUE".to_string())
            }
            Token::False => {
                self.advance();
                Ok("FALSE".to_string())
            }
            Token::OpenParen => {
                self.advance();
                // Could be a subquery or expression
                if self.peek_is(&Token::Select) {
                    let subquery = self.parse_select()?;
                    self.expect(&Token::CloseParen)?;
                    let idx = self.nested_queries.len();
                    self.nested_queries.push(Box::new(subquery));
                    Ok(format!("(SELECT __subq_{}__)", idx))
                } else {
                    let expr = self.parse_expression()?;
                    self.expect(&Token::CloseParen)?;
                    Ok(format!("({})", expr))
                }
            }
            Token::Star => {
                self.advance();
                Ok("*".to_string())
            }
            Token::Identifier(_) => {
                let mut name = self.parse_identifier()?;

                // Function call
                if self.peek_is(&Token::OpenParen) {
                    self.advance();
                    let mut args = Vec::new();
                    if !self.peek_is(&Token::CloseParen) {
                        loop {
                            args.push(self.parse_expression()?);
                            if self.peek_is(&Token::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(&Token::CloseParen)?;

                    // Check for window function (OVER clause)
                    if self.peek_is(&Token::Over) {
                        let window = self.parse_window_spec()?;
                        let _alias: Option<String> = None; // alias is handled by parse_select_column
                        self.window_functions.push(WindowFunctionColumn {
                            function: name.clone(),
                            args: args.clone(),
                            window: window.clone(),
                            alias: None,
                        });
                        let window_str = format_window_spec(&window);
                        return Ok(format!("{}({}) OVER {}", name, args.join(", "), window_str));
                    }

                    return Ok(format!("{}({})", name, args.join(", ")));
                }

                // Qualified identifier (table.column)
                if self.peek_is(&Token::Dot) {
                    self.advance();
                    let col = self.parse_identifier()?;
                    name = format!("{}.{}", name, col);
                    if self.peek_is(&Token::OpenParen) {
                        self.advance();
                        let mut args = Vec::new();
                        if !self.peek_is(&Token::CloseParen) {
                            loop {
                                args.push(self.parse_expression()?);
                                if self.peek_is(&Token::Comma) {
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                        }
                        self.expect(&Token::CloseParen)?;
                        name = format!("{}({})", name, args.join(", "));

                        if self.peek_is(&Token::Over) {
                            let window = self.parse_window_spec()?;
                            self.window_functions.push(WindowFunctionColumn {
                                function: name.clone(),
                                args: vec![],
                                window: window.clone(),
                                alias: None,
                            });
                            let window_str = format_window_spec(&window);
                            return Ok(format!("{} OVER {}", name, window_str));
                        }
                    }
                }

                Ok(name)
            }
            _ => Err(crate::Error::ValidationError(format!(
                "Unexpected token in expression: {:?}",
                self.peek()
            ))),
        }
    }

    fn parse_identifier(&mut self) -> Result<String> {
        match self.advance() {
            Token::Identifier(s) => Ok(s),
            Token::Star => Ok("*".to_string()),
            tok => {
                // Accept keyword tokens as identifiers (e.g. COLUMN, INDEX, KEY, etc.)
                match tok {
                    Token::Eof
                    | Token::Comma
                    | Token::Dot
                    | Token::Semicolon
                    | Token::OpenParen
                    | Token::CloseParen
                    | Token::Eq
                    | Token::Neq
                    | Token::Gt
                    | Token::Gte
                    | Token::Lt
                    | Token::Lte
                    | Token::Plus
                    | Token::Minus
                    | Token::Slash
                    | Token::Percent
                    | Token::Assignment => Err(crate::Error::ValidationError(format!(
                        "Expected identifier, found {:?}",
                        tok
                    ))),
                    _ => Ok(format!("{}", tok)),
                }
            }
        }
    }

    fn parse_number(&mut self) -> Result<usize> {
        match self.advance() {
            Token::Number(s) => s
                .parse::<usize>()
                .map_err(|e| crate::Error::ValidationError(format!("Invalid number: {}", e))),
            Token::Identifier(s) => s
                .parse::<usize>()
                .map_err(|e| crate::Error::ValidationError(format!("Invalid number: {}", e))),
            other => Err(crate::Error::ValidationError(format!(
                "Expected number, found {:?}",
                other
            ))),
        }
    }

    fn parse_value(&mut self) -> Result<String> {
        match self.peek() {
            Token::Number(_) => {
                let n = self.advance();
                if let Token::Number(s) = n {
                    Ok(s)
                } else {
                    Err(crate::Error::ValidationError(format!(
                        "Expected number, found {:?}",
                        n
                    )))
                }
            }
            Token::String(_) => {
                let s = self.advance();
                if let Token::String(s) = s {
                    Ok(format!("'{}'", s))
                } else {
                    Err(crate::Error::ValidationError(format!(
                        "Expected string, found {:?}",
                        s
                    )))
                }
            }
            Token::Null => {
                self.advance();
                Ok("NULL".to_string())
            }
            Token::True => {
                self.advance();
                Ok("TRUE".to_string())
            }
            Token::False => {
                self.advance();
                Ok("FALSE".to_string())
            }
            Token::Minus => {
                self.advance();
                let n = self.parse_number()?;
                Ok(format!("-{}", n))
            }
            Token::Identifier(_) => self.parse_identifier(),
            _ => Err(crate::Error::ValidationError(format!(
                "Expected value, found {:?}",
                self.peek()
            ))),
        }
    }

    fn extract_aggregations(&self, columns: &[String]) -> Vec<AggregationClause> {
        let mut aggs = Vec::new();
        let agg_keywords = [
            "COUNT(",
            "SUM(",
            "AVG(",
            "MIN(",
            "MAX(",
            "GROUP_CONCAT(",
            "ARRAY_AGG(",
        ];
        for col in columns {
            let upper = col.to_uppercase();
            for &kw in &agg_keywords {
                if upper.contains(kw) {
                    let inner = col[col.find('(').map(|i| i + 1).unwrap_or(0)
                        ..col.rfind(')').unwrap_or(col.len())]
                        .trim()
                        .to_string();
                    let alias = col.find(" AS ").map(|i| col[i + 4..].trim().to_string());
                    let agg_type = match kw {
                        "COUNT(" => AggregationType::Count,
                        "SUM(" => AggregationType::Sum,
                        "AVG(" => AggregationType::Avg,
                        "MIN(" => AggregationType::Min,
                        "MAX(" => AggregationType::Max,
                        "GROUP_CONCAT(" => AggregationType::GroupConcat,
                        "ARRAY_AGG(" => AggregationType::ArrayAgg,
                        _ => continue,
                    };
                    aggs.push(AggregationClause {
                        agg_type,
                        column: inner,
                        alias,
                    });
                    break;
                }
            }
        }
        aggs
    }
}

// ── sqlparser-rs Adapter ────────────────────────────────────────
// Replaces the ad-hoc SQL parser with sqlparser-rs for standard SQL.
// Falls back to the old parser for edge cases or extended syntax.

use sqlparser::ast::{
    self, Expr, GroupByExpr, ObjectName, ObjectType, Select, SelectItem, SetExpr, Statement,
    TableFactor, TableWithJoins, Value,
};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser as SqlParser;

fn parse_with_sqlparser(sql: &str) -> Result<ParsedQuery> {
    let dialect = GenericDialect {};
    let statements = SqlParser::parse_sql(&dialect, sql)
        .map_err(|e| crate::Error::ValidationError(format!("SQL parse error: {}", e)))?;

    if statements.len() != 1 {
        return Err(crate::Error::ValidationError(
            "Only single SQL statements are supported".to_string(),
        ));
    }

    let stmt = statements.into_iter().next().unwrap();
    convert_statement(stmt)
}

fn convert_statement(stmt: Statement) -> Result<ParsedQuery> {
    match stmt {
        Statement::Query(query) => convert_query(*query),
        Statement::Insert(insert) => convert_insert(insert),
        Statement::Update {
            table,
            assignments,
            selection,
            ..
        } => convert_update(table, assignments, selection),
        Statement::Delete(delete) => convert_delete(delete),
        Statement::CreateTable(create) => convert_create_table(create),
        Statement::Drop {
            object_type, names, ..
        } => convert_drop(object_type, names),
        _ => Err(crate::Error::ValidationError(
            "Unsupported SQL statement type".to_string(),
        )),
    }
}

fn convert_query(query: sqlparser::ast::Query) -> Result<ParsedQuery> {
    let mut ctes = Vec::new();
    if let Some(ref with) = query.with {
        for cte in &with.cte_tables {
            let cte_query = convert_query(*cte.query.clone())?;
            let columns = if cte.alias.columns.is_empty() {
                None
            } else {
                Some(cte.alias.columns.iter().map(|c| c.value.clone()).collect())
            };
            ctes.push(CTEDefinition {
                name: cte.alias.name.value.clone(),
                columns,
                query: Box::new(cte_query),
            });
        }
    }

    let order_by: Vec<OrderByClause> = query
        .order_by
        .as_ref()
        .map(|o| o.exprs.iter().map(order_by_to_clause).collect())
        .unwrap_or_default();

    let limit = query
        .limit
        .as_ref()
        .and_then(expr_to_u64)
        .map(|v| v as usize);
    let offset = query
        .offset
        .as_ref()
        .and_then(|o| expr_to_u64(&o.value))
        .map(|v| v as usize);

    match &*query.body {
        SetExpr::Select(select) => {
            let mut pq = convert_select(select.as_ref().clone())?;
            pq.ctes = ctes;
            pq.order_by = order_by;
            pq.limit = limit;
            pq.offset = offset;
            Ok(pq)
        }
        SetExpr::SetOperation { .. } => Err(crate::Error::ValidationError(
            "Set operations (UNION/INTERSECT/EXCEPT) not yet supported".to_string(),
        )),
        _ => Err(crate::Error::ValidationError(
            "Unsupported query expression".to_string(),
        )),
    }
}

fn expr_to_u64(expr: &Expr) -> Option<u64> {
    match expr {
        Expr::Value(Value::Number(n, _)) => n.parse::<u64>().ok(),
        _ => None,
    }
}

fn group_by_to_strings(gb: &GroupByExpr) -> Vec<String> {
    match gb {
        GroupByExpr::Expressions(exprs, _) => exprs.iter().map(expr_to_string).collect(),
        GroupByExpr::All(_) => vec!["ALL".to_string()],
    }
}

fn has_window_function(expr: &Expr) -> bool {
    match expr {
        Expr::Function(f) => f.over.is_some(),
        Expr::Nested(e) => has_window_function(e),
        Expr::BinaryOp { left, right, .. } => {
            has_window_function(left) || has_window_function(right)
        }
        Expr::UnaryOp { expr: e, .. } => has_window_function(e),
        _ => false,
    }
}

fn convert_select(select: Select) -> Result<ParsedQuery> {
    // Window functions are not yet supported by the sqlparser adapter —
    // fall back to the old parser for these.
    for item in &select.projection {
        if let SelectItem::UnnamedExpr(expr) = item {
            if has_window_function(expr) {
                return Err(crate::Error::ValidationError(
                    "Window functions not supported in sqlparser adapter".to_string(),
                ));
            }
        }
        if let SelectItem::ExprWithAlias { expr, .. } = item {
            if has_window_function(expr) {
                return Err(crate::Error::ValidationError(
                    "Window functions not supported in sqlparser adapter".to_string(),
                ));
            }
        }
    }

    let distinct = select.distinct.is_some();

    let mut source_tables = Vec::new();
    let mut joins = Vec::new();

    for table_with_joins in &select.from {
        if let TableFactor::Table { name, .. } = &table_with_joins.relation {
            source_tables.push(object_name_to_string(name));
        }

        for join in &table_with_joins.joins {
            let join_table = match &join.relation {
                TableFactor::Table { name, .. } => object_name_to_string(name),
                _ => continue,
            };

            let join_type = match &join.join_operator {
                ast::JoinOperator::Inner(_) => JoinType::Inner,
                ast::JoinOperator::LeftOuter(_) => JoinType::Left,
                ast::JoinOperator::RightOuter(_) => JoinType::Right,
                ast::JoinOperator::FullOuter(_) => JoinType::Full,
                ast::JoinOperator::CrossJoin | ast::JoinOperator::CrossApply => JoinType::Cross,
                _ => JoinType::Inner,
            };

            let condition = match &join.join_operator {
                ast::JoinOperator::Inner(c)
                | ast::JoinOperator::LeftOuter(c)
                | ast::JoinOperator::RightOuter(c)
                | ast::JoinOperator::FullOuter(c) => match c {
                    ast::JoinConstraint::On(expr) => expr_to_string(expr),
                    _ => String::new(),
                },
                _ => String::new(),
            };

            joins.push(JoinClause {
                join_type,
                table: join_table,
                condition,
                engine_hint: None,
            });
        }
    }

    let columns: Vec<String> = select
        .projection
        .iter()
        .map(select_item_to_string)
        .collect();
    let conditions = select.selection.as_ref().map(expr_to_string);
    let group_by = group_by_to_strings(&select.group_by);
    let having = select.having.as_ref().map(expr_to_string);
    let aggregations = extract_aggregations_from_columns(&columns);

    Ok(ParsedQuery {
        operation: QueryOperation::Select,
        source_tables,
        target_table: None,
        columns,
        conditions,
        joins,
        order_by: vec![],
        group_by,
        aggregations,
        limit: None,
        offset: None,
        set_operations: vec![],
        nested_queries: vec![],
        distinct,
        having,
        ctes: vec![],
        window_functions: vec![],
    })
}

fn convert_insert(insert: sqlparser::ast::Insert) -> Result<ParsedQuery> {
    let target_table = Some(object_name_to_string(&insert.table_name));

    let columns: Vec<String> = insert.columns.iter().map(|c| c.value.clone()).collect();

    let conditions = insert
        .source
        .as_ref()
        .map(|source| match source.body.as_ref() {
            SetExpr::Values(values) => {
                let rows: Vec<String> = values
                    .rows
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(expr_to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .collect();
                rows.join("; ")
            }
            _ => "(SELECT ...)".to_string(),
        });

    Ok(ParsedQuery {
        operation: QueryOperation::Insert,
        source_tables: vec![],
        target_table,
        columns,
        conditions,
        joins: vec![],
        order_by: vec![],
        group_by: vec![],
        aggregations: vec![],
        limit: None,
        offset: None,
        set_operations: vec![],
        nested_queries: vec![],
        distinct: false,
        having: None,
        ctes: vec![],
        window_functions: vec![],
    })
}

fn convert_update(
    table: TableWithJoins,
    assignments: Vec<ast::Assignment>,
    selection: Option<Expr>,
) -> Result<ParsedQuery> {
    let target_table = match &table.relation {
        TableFactor::Table { name, .. } => Some(object_name_to_string(name)),
        _ => {
            return Err(crate::Error::ValidationError(
                "UPDATE target must be a table".to_string(),
            ))
        }
    };

    let set_clauses: Vec<String> = assignments
        .iter()
        .map(|a| {
            let col = format!("{}", a.target);
            let val = expr_to_string(&a.value);
            format!("{} = {}", col, val)
        })
        .collect();

    let conditions = selection.as_ref().map(expr_to_string);

    Ok(ParsedQuery {
        operation: QueryOperation::Update,
        source_tables: vec![],
        target_table,
        columns: set_clauses,
        conditions,
        joins: vec![],
        order_by: vec![],
        group_by: vec![],
        aggregations: vec![],
        limit: None,
        offset: None,
        set_operations: vec![],
        nested_queries: vec![],
        distinct: false,
        having: None,
        ctes: vec![],
        window_functions: vec![],
    })
}

fn convert_delete(delete: ast::Delete) -> Result<ParsedQuery> {
    let target_table = if !delete.tables.is_empty() {
        Some(object_name_to_string(&delete.tables[0]))
    } else {
        let tables = match &delete.from {
            ast::FromTable::WithFromKeyword(tables) | ast::FromTable::WithoutKeyword(tables) => {
                tables
            }
        };
        tables.first().and_then(|t| match &t.relation {
            TableFactor::Table { name, .. } => Some(object_name_to_string(name)),
            _ => None,
        })
    };

    let conditions = delete.selection.as_ref().map(expr_to_string);

    Ok(ParsedQuery {
        operation: QueryOperation::Delete,
        source_tables: vec![],
        target_table,
        columns: vec![],
        conditions,
        joins: vec![],
        order_by: vec![],
        group_by: vec![],
        aggregations: vec![],
        limit: None,
        offset: None,
        set_operations: vec![],
        nested_queries: vec![],
        distinct: false,
        having: None,
        ctes: vec![],
        window_functions: vec![],
    })
}

fn convert_create_table(create: ast::CreateTable) -> Result<ParsedQuery> {
    let target_table = Some(object_name_to_string(&create.name));

    let columns: Vec<String> = create
        .columns
        .iter()
        .map(|col| format!("{} {}", col.name.value, col.data_type))
        .collect();

    Ok(ParsedQuery {
        operation: QueryOperation::Create,
        source_tables: vec![],
        target_table,
        columns,
        conditions: None,
        joins: vec![],
        order_by: vec![],
        group_by: vec![],
        aggregations: vec![],
        limit: None,
        offset: None,
        set_operations: vec![],
        nested_queries: vec![],
        distinct: false,
        having: None,
        ctes: vec![],
        window_functions: vec![],
    })
}

fn convert_drop(object_type: ObjectType, names: Vec<ObjectName>) -> Result<ParsedQuery> {
    if !matches!(object_type, ObjectType::Table) {
        return Err(crate::Error::ValidationError(format!(
            "Unsupported DROP object type: {:?}",
            object_type
        )));
    }

    let target_table = names.first().map(object_name_to_string);

    Ok(ParsedQuery {
        operation: QueryOperation::Drop,
        source_tables: vec![],
        target_table,
        columns: vec![],
        conditions: None,
        joins: vec![],
        order_by: vec![],
        group_by: vec![],
        aggregations: vec![],
        limit: None,
        offset: None,
        set_operations: vec![],
        nested_queries: vec![],
        distinct: false,
        having: None,
        ctes: vec![],
        window_functions: vec![],
    })
}

// ── Expression / Item conversion helpers ─────────────────────────

fn expr_to_string(expr: &Expr) -> String {
    match expr {
        Expr::Identifier(ident) => ident.value.clone(),
        Expr::CompoundIdentifier(idents) => idents
            .iter()
            .map(|i| i.value.clone())
            .collect::<Vec<_>>()
            .join("."),
        Expr::Wildcard => "*".to_string(),
        Expr::QualifiedWildcard(ids) => format!(
            "{}.*",
            ids.0
                .iter()
                .map(|i| i.value.clone())
                .collect::<Vec<_>>()
                .join(".")
        ),
        Expr::Value(v) => value_to_string(v),
        Expr::BinaryOp { left, op, right } => {
            format!("{} {} {}", expr_to_string(left), op, expr_to_string(right))
        }
        Expr::UnaryOp { op, expr: e } => {
            format!("{}{}", op, expr_to_string(e))
        }
        Expr::Nested(inner) => format!("({})", expr_to_string(inner)),
        Expr::Function(func) => {
            let args = function_args_to_strings(&func.args);
            format!("{}({})", func.name, args.join(", "))
        }
        Expr::InList {
            expr: e,
            list,
            negated,
        } => {
            let items: Vec<String> = list.iter().map(expr_to_string).collect();
            if *negated {
                format!("{} NOT IN ({})", expr_to_string(e), items.join(", "))
            } else {
                format!("{} IN ({})", expr_to_string(e), items.join(", "))
            }
        }
        Expr::InSubquery {
            expr: e,
            subquery: _,
            negated,
        } => {
            if *negated {
                format!("{} NOT IN (SELECT ...)", expr_to_string(e))
            } else {
                format!("{} IN (SELECT ...)", expr_to_string(e))
            }
        }
        Expr::Between {
            expr: e,
            negated,
            low,
            high,
        } => {
            if *negated {
                format!(
                    "{} NOT BETWEEN {} AND {}",
                    expr_to_string(e),
                    expr_to_string(low),
                    expr_to_string(high)
                )
            } else {
                format!(
                    "{} BETWEEN {} AND {}",
                    expr_to_string(e),
                    expr_to_string(low),
                    expr_to_string(high)
                )
            }
        }
        Expr::Like {
            expr: e,
            negated,
            pattern,
            ..
        } => {
            if *negated {
                format!("{} NOT LIKE {}", expr_to_string(e), expr_to_string(pattern))
            } else {
                format!("{} LIKE {}", expr_to_string(e), expr_to_string(pattern))
            }
        }
        Expr::ILike {
            expr: e,
            negated,
            pattern,
            ..
        } => {
            if *negated {
                format!(
                    "{} NOT ILIKE {}",
                    expr_to_string(e),
                    expr_to_string(pattern)
                )
            } else {
                format!("{} ILIKE {}", expr_to_string(e), expr_to_string(pattern))
            }
        }
        Expr::IsNull(e) => format!("{} IS NULL", expr_to_string(e)),
        Expr::IsNotNull(e) => format!("{} IS NOT NULL", expr_to_string(e)),
        Expr::InUnnest { expr: e, .. } => format!("{} IN (SELECT ...)", expr_to_string(e)),
        Expr::Subquery(_) => "(SELECT ...)".to_string(),
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            let mut out = "CASE".to_string();
            if let Some(op) = operand {
                out.push(' ');
                out.push_str(&expr_to_string(op));
            }
            for (cond, res) in conditions.iter().zip(results.iter()) {
                out.push_str(&format!(
                    " WHEN {} THEN {}",
                    expr_to_string(cond),
                    expr_to_string(res)
                ));
            }
            if let Some(els) = else_result {
                out.push_str(&format!(" ELSE {}", expr_to_string(els)));
            }
            out.push_str(" END");
            out
        }
        Expr::Cast {
            expr: e, data_type, ..
        } => {
            format!("CAST({} AS {})", expr_to_string(e), data_type)
        }
        Expr::Extract { field, expr: e, .. } => {
            format!("EXTRACT({} FROM {})", field, expr_to_string(e))
        }
        Expr::Exists { subquery, .. } => {
            let _ = subquery;
            "EXISTS (SELECT ...)".to_string()
        }
        _ => format!("{:?}", expr),
    }
}

fn function_args_to_strings(args: &sqlparser::ast::FunctionArguments) -> Vec<String> {
    match args {
        sqlparser::ast::FunctionArguments::None => vec![],
        sqlparser::ast::FunctionArguments::Subquery(_) => vec!["(SELECT ...)".to_string()],
        sqlparser::ast::FunctionArguments::List(list) => list
            .args
            .iter()
            .map(|arg| match arg {
                sqlparser::ast::FunctionArg::Unnamed(expr) => function_arg_expr_to_string(expr),
                sqlparser::ast::FunctionArg::Named {
                    name, arg: expr, ..
                } => format!("{} => {}", name.value, function_arg_expr_to_string(expr)),
            })
            .collect(),
    }
}

fn function_arg_expr_to_string(expr: &sqlparser::ast::FunctionArgExpr) -> String {
    match expr {
        sqlparser::ast::FunctionArgExpr::Expr(e) => expr_to_string(e),
        sqlparser::ast::FunctionArgExpr::QualifiedWildcard(name) => {
            format!("{}.*", object_name_to_string(name))
        }
        sqlparser::ast::FunctionArgExpr::Wildcard => "*".to_string(),
    }
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Number(n, _) => n.clone(),
        Value::SingleQuotedString(s) => format!("'{}'", s.replace('\'', "''")),
        Value::DoubleQuotedString(s) => format!("\"{}\"", s),
        Value::Boolean(b) => {
            if *b {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        Value::Null => "NULL".to_string(),
        Value::Placeholder(param) => format!("${}", param),
        _ => format!("{:?}", value),
    }
}

fn select_item_to_string(item: &SelectItem) -> String {
    match item {
        SelectItem::UnnamedExpr(expr) => expr_to_string(expr),
        SelectItem::ExprWithAlias { expr, alias } => {
            format!("{} AS {}", expr_to_string(expr), alias.value)
        }
        SelectItem::QualifiedWildcard(name, _) => {
            format!(
                "{}.*",
                name.0
                    .iter()
                    .map(|i| i.value.clone())
                    .collect::<Vec<_>>()
                    .join(".")
            )
        }
        SelectItem::Wildcard(_) => "*".to_string(),
    }
}

fn object_name_to_string(name: &ObjectName) -> String {
    name.0
        .iter()
        .map(|i| i.value.clone())
        .collect::<Vec<_>>()
        .join(".")
}

fn order_by_to_clause(ob: &sqlparser::ast::OrderByExpr) -> OrderByClause {
    let column = expr_to_string(&ob.expr);
    let direction = match ob.asc {
        Some(true) | None => "ASC",
        Some(false) => "DESC",
    };
    OrderByClause {
        column,
        direction: direction.to_string(),
    }
}

fn extract_aggregations_from_columns(columns: &[String]) -> Vec<AggregationClause> {
    let agg_keywords: [(&str, AggregationType); 7] = [
        ("COUNT(", AggregationType::Count),
        ("SUM(", AggregationType::Sum),
        ("AVG(", AggregationType::Avg),
        ("MIN(", AggregationType::Min),
        ("MAX(", AggregationType::Max),
        ("GROUP_CONCAT(", AggregationType::GroupConcat),
        ("ARRAY_AGG(", AggregationType::ArrayAgg),
    ];

    let mut aggs = Vec::new();
    for col in columns {
        let upper = col.to_uppercase();
        for &(kw, ref agg_type) in &agg_keywords {
            if upper.contains(kw) {
                let inner = col[col.find('(').map(|i| i + 1).unwrap_or(0)
                    ..col.rfind(')').unwrap_or(col.len())]
                    .trim()
                    .to_string();
                let alias = col.find(" AS ").map(|i| col[i + 4..].trim().to_string());
                aggs.push(AggregationClause {
                    agg_type: agg_type.clone(),
                    column: inner,
                    alias,
                });
                break;
            }
        }
    }
    aggs
}

// ── Public API ──────────────────────────────────────────────────

/// Query parser for multiple query languages
pub struct QueryParser;

impl Default for QueryParser {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryParser {
    pub fn new() -> Self {
        QueryParser
    }

    pub fn parse(&self, query: &UqlQuery) -> Result<ParsedQuery> {
        match query.query_type {
            QueryLanguage::Sql => self.parse_sql(&query.query),
            QueryLanguage::MongoDb => self.parse_mongodb(&query.query),
            QueryLanguage::Mango => self.parse_mango(&query.query),
            QueryLanguage::Uql => self.parse_uql(&query.query),
            QueryLanguage::Auto => self.detect_and_parse(&query.query),
        }
    }

    fn detect_and_parse(&self, query: &str) -> Result<ParsedQuery> {
        let trimmed = query.trim();

        if trimmed.starts_with("SELECT")
            || trimmed.starts_with("select")
            || trimmed.starts_with("INSERT")
            || trimmed.starts_with("insert")
            || trimmed.starts_with("UPDATE")
            || trimmed.starts_with("update")
            || trimmed.starts_with("DELETE")
            || trimmed.starts_with("delete")
            || trimmed.starts_with("CREATE")
            || trimmed.starts_with("create")
            || trimmed.starts_with("DROP")
            || trimmed.starts_with("drop")
            || trimmed.starts_with("ALTER")
            || trimmed.starts_with("alter")
        {
            return self.parse_sql(trimmed);
        }

        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
                // Mango format has "selector" at top level
                if json.get("selector").is_some() {
                    return self.parse_mango(trimmed);
                }
                // MongoDB format has collection names as top-level keys
                // UQL format has "from", "select", "where" etc.
                if json.get("from").is_some() || json.get("select").is_some() {
                    return self.parse_uql(trimmed);
                }
                return self.parse_mongodb(trimmed);
            }
        }

        self.parse_uql(trimmed)
    }

    fn parse_sql(&self, query: &str) -> Result<ParsedQuery> {
        // Try sqlparser-rs first for standard SQL; fall back to the old parser.
        if let Ok(pq) = parse_with_sqlparser(query) {
            return Ok(pq);
        }
        let mut tokenizer = Tokenizer::new(query);
        let tokens = tokenizer.tokenize()?;
        let mut parser = Parser::new(tokens);
        parser.parse_query()
    }

    fn parse_mongodb(&self, query: &str) -> Result<ParsedQuery> {
        let value: serde_json::Value = serde_json::from_str(query)
            .map_err(|e| crate::Error::ValidationError(format!("Invalid JSON: {}", e)))?;

        let mut parsed = ParsedQuery {
            operation: QueryOperation::Select,
            source_tables: vec![],
            target_table: None,
            columns: vec!["*".to_string()],
            conditions: None,
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        };

        if let Some(obj) = value.as_object() {
            let mut all_conditions = Vec::new();
            for (key, val) in obj {
                if key.starts_with('$') {
                    // Top-level operator: treat as condition group
                    if let Some(sql) = Self::json_operator_to_sql(key, val) {
                        all_conditions.push(sql);
                    }
                } else {
                    parsed.source_tables.push(key.clone());
                    // The value for a table is a filter object
                    if let Some(filter_obj) = val.as_object() {
                        let cond = Self::json_object_to_sql_conditions(key, filter_obj);
                        if !cond.is_empty() {
                            all_conditions.push(cond);
                        }
                    } else {
                        // Direct equality: {"users": "active"}
                        let val_sql = Self::json_value_to_sql_literal(val);
                        all_conditions.push(format!("{} = {}", key, val_sql));
                    }
                }
            }
            if !all_conditions.is_empty() {
                parsed.conditions = Some(all_conditions.join(" AND "));
            }
        }

        Ok(parsed)
    }

    fn parse_mango(&self, query: &str) -> Result<ParsedQuery> {
        let value: serde_json::Value = serde_json::from_str(query)
            .map_err(|e| crate::Error::ValidationError(format!("Invalid JSON: {}", e)))?;

        let mut parsed = ParsedQuery {
            operation: QueryOperation::Select,
            source_tables: vec![],
            target_table: None,
            columns: vec!["*".to_string()],
            conditions: None,
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        };

        if let Some(obj) = value.as_object() {
            if let Some(selector) = obj.get("selector") {
                parsed.conditions = Self::json_mango_selector_to_sql(selector);
            }

            if let Some(limit_val) = obj.get("limit").and_then(|v| v.as_u64()) {
                parsed.limit = Some(limit_val as usize);
            }

            if let Some(skip_val) = obj.get("skip").and_then(|v| v.as_u64()) {
                parsed.offset = Some(skip_val as usize);
            }

            if let Some(sort_arr) = obj.get("sort").and_then(|v| v.as_array()) {
                for item in sort_arr {
                    if let Some(obj) = item.as_object() {
                        for (col, dir) in obj {
                            parsed.order_by.push(OrderByClause {
                                column: col.clone(),
                                direction: if dir.as_str() == Some("desc") {
                                    "DESC".to_string()
                                } else {
                                    "ASC".to_string()
                                },
                            });
                        }
                    }
                }
            }

            // Use fields as projection
            if let Some(fields) = obj.get("fields").and_then(|v| v.as_array()) {
                let cols: Vec<String> = fields
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                if !cols.is_empty() {
                    parsed.columns = cols;
                }
            }

            // Extract index hint
            if let Some(index_name) = obj.get("use_index").and_then(|v| v.as_str()) {
                parsed.conditions = parsed
                    .conditions
                    .map(|c| format!("{} /* use_index:{} */", c, index_name));
            }
        }

        Ok(parsed)
    }

    /// Convert a Mango selector JSON to a SQL-like condition string
    fn json_mango_selector_to_sql(selector: &serde_json::Value) -> Option<String> {
        match selector {
            serde_json::Value::Object(obj) => {
                let mut parts = Vec::new();
                for (key, val) in obj {
                    if key.starts_with('$') {
                        if let Some(sql) = Self::json_operator_to_sql(key, val) {
                            parts.push(sql);
                        }
                    } else {
                        // Field-based condition
                        match val {
                            serde_json::Value::Object(op_obj) => {
                                let cond = Self::json_field_conditions_to_sql(key, op_obj);
                                if !cond.is_empty() {
                                    parts.push(cond);
                                }
                            }
                            _ => {
                                // Direct equality: { "status": "active" }
                                let val_sql = Self::json_value_to_sql_literal(val);
                                parts.push(format!("{} = {}", key, val_sql));
                            }
                        }
                    }
                }
                if parts.is_empty() {
                    None
                } else if parts.len() == 1 {
                    Some(parts.remove(0))
                } else {
                    Some(format!("({})", parts.join(" AND ")))
                }
            }
            _ => None,
        }
    }

    /// Convert a field's operator object to SQL conditions
    /// e.g. {"$gt": 25, "$lt": 100} -> "age > 25 AND age < 100"
    fn json_field_conditions_to_sql(
        field: &str,
        obj: &serde_json::Map<String, serde_json::Value>,
    ) -> String {
        let mut parts = Vec::new();
        for (op, val) in obj {
            let val_sql = Self::json_value_to_sql_literal(val);
            let cond = match op.as_str() {
                "$eq" => format!("{} = {}", field, val_sql),
                "$ne" => format!("{} != {}", field, val_sql),
                "$gt" => format!("{} > {}", field, val_sql),
                "$gte" => format!("{} >= {}", field, val_sql),
                "$lt" => format!("{} < {}", field, val_sql),
                "$lte" => format!("{} <= {}", field, val_sql),
                "$in" => {
                    if let Some(arr) = val.as_array() {
                        let items: Vec<String> =
                            arr.iter().map(Self::json_value_to_sql_literal).collect();
                        format!("{} IN ({})", field, items.join(", "))
                    } else {
                        format!("{} IN ({})", field, val_sql)
                    }
                }
                "$nin" => {
                    if let Some(arr) = val.as_array() {
                        let items: Vec<String> =
                            arr.iter().map(Self::json_value_to_sql_literal).collect();
                        format!("{} NOT IN ({})", field, items.join(", "))
                    } else {
                        format!("{} NOT IN ({})", field, val_sql)
                    }
                }
                "$exists" => {
                    if val.as_bool().unwrap_or(false) {
                        format!("{} IS NOT NULL", field)
                    } else {
                        format!("{} IS NULL", field)
                    }
                }
                "$regex" => {
                    let pattern = val.as_str().unwrap_or("");
                    format!(
                        "{} LIKE '%{}%'",
                        field,
                        pattern.trim_matches('^').trim_matches('$')
                    )
                }
                "$not" => {
                    if let Some(inner) = val.as_object() {
                        let inner_cond = Self::json_field_conditions_to_sql(field, inner);
                        format!("NOT ({})", inner_cond)
                    } else {
                        format!("NOT ({} = {})", field, val_sql)
                    }
                }
                "$all" => {
                    if let Some(arr) = val.as_array() {
                        let items: Vec<String> = arr
                            .iter()
                            .map(|v| {
                                format!("{} LIKE '%{}%'", field, Self::json_value_to_sql_literal(v))
                            })
                            .collect();
                        items.join(" AND ")
                    } else {
                        format!("{} LIKE '%{}%'", field, val_sql)
                    }
                }
                _ => format!("{} {} {}", field, op, val_sql),
            };
            parts.push(cond);
        }
        parts.join(" AND ")
    }

    /// Convert a top-level operator like $and, $or, $nor to SQL
    fn json_operator_to_sql(op: &str, val: &serde_json::Value) -> Option<String> {
        match op {
            "$and" => {
                if let Some(arr) = val.as_array() {
                    let parts: Vec<String> = arr
                        .iter()
                        .filter_map(Self::json_mango_selector_to_sql)
                        .collect();
                    if parts.is_empty() {
                        None
                    } else {
                        Some(format!("({})", parts.join(" AND ")))
                    }
                } else {
                    None
                }
            }
            "$or" => {
                if let Some(arr) = val.as_array() {
                    let parts: Vec<String> = arr
                        .iter()
                        .filter_map(Self::json_mango_selector_to_sql)
                        .collect();
                    if parts.is_empty() {
                        None
                    } else {
                        Some(format!("({})", parts.join(" OR ")))
                    }
                } else {
                    None
                }
            }
            "$nor" => {
                if let Some(arr) = val.as_array() {
                    let parts: Vec<String> = arr
                        .iter()
                        .filter_map(Self::json_mango_selector_to_sql)
                        .collect();
                    if parts.is_empty() {
                        None
                    } else {
                        Some(format!("NOT ({})", parts.join(" OR ")))
                    }
                } else {
                    None
                }
            }
            "$not" => {
                if let Some(inner) = val.as_object() {
                    let cond = Self::json_object_to_sql_conditions("_", inner);
                    if cond.is_empty() {
                        None
                    } else {
                        Some(format!("NOT ({})", cond))
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Convert a JSON object of field->conditions to SQL string
    fn json_object_to_sql_conditions(
        _table_or_field: &str,
        obj: &serde_json::Map<String, serde_json::Value>,
    ) -> String {
        let mut parts = Vec::new();
        for (key, val) in obj {
            if key.starts_with('$') {
                if let Some(sql) = Self::json_operator_to_sql(key, val) {
                    parts.push(sql);
                }
            } else {
                match val {
                    serde_json::Value::Object(op_obj) => {
                        let cond = Self::json_field_conditions_to_sql(key, op_obj);
                        if !cond.is_empty() {
                            parts.push(cond);
                        }
                    }
                    _ => {
                        let val_sql = Self::json_value_to_sql_literal(val);
                        parts.push(format!("{} = {}", key, val_sql));
                    }
                }
            }
        }
        if parts.len() == 1 {
            parts.remove(0)
        } else if parts.is_empty() {
            String::new()
        } else {
            format!("({})", parts.join(" AND "))
        }
    }

    /// Convert a JSON value to a SQL literal string
    fn json_value_to_sql_literal(val: &serde_json::Value) -> String {
        match val {
            serde_json::Value::Null => "NULL".to_string(),
            serde_json::Value::Bool(b) => {
                if *b {
                    "TRUE".to_string()
                } else {
                    "FALSE".to_string()
                }
            }
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => {
                let escaped = s.replace('\'', "''");
                format!("'{}'", escaped)
            }
            serde_json::Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(Self::json_value_to_sql_literal).collect();
                format!("({})", items.join(", "))
            }
            serde_json::Value::Object(_) => {
                // Nested object as JSON string literal
                format!("'{}'", val.to_string().replace('\'', "''"))
            }
        }
    }

    fn parse_uql(&self, query: &str) -> Result<ParsedQuery> {
        let value: serde_json::Value = serde_json::from_str(query)
            .map_err(|e| crate::Error::ValidationError(format!("Invalid UQL: {}", e)))?;

        let mut parsed = ParsedQuery {
            operation: QueryOperation::Select,
            source_tables: vec![],
            target_table: None,
            columns: vec!["*".to_string()],
            conditions: None,
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        };

        if let Some(obj) = value.as_object() {
            if let Some(from) = obj.get("from").and_then(|v| v.as_str()) {
                parsed.source_tables.push(from.to_string());
            }

            if let Some(select) = obj.get("select") {
                if let Some(cols) = select.as_array() {
                    parsed.columns = cols
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                }
            }

            if let Some(where_cond) = obj.get("where") {
                parsed.conditions = Some(where_cond.to_string());
            }

            if let Some(joins) = obj.get("joins").and_then(|v| v.as_array()) {
                for join_val in joins {
                    if let Some(join_obj) = join_val.as_object() {
                        let join = JoinClause {
                            join_type: JoinType::Inner,
                            table: join_obj
                                .get("table")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            condition: join_obj
                                .get("on")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            engine_hint: None,
                        };
                        parsed.joins.push(join);
                    }
                }
            }

            if let Some(limit_val) = obj.get("limit").and_then(|v| v.as_u64()) {
                parsed.limit = Some(limit_val as usize);
            }
        }

        Ok(parsed)
    }
}

fn format_window_spec(window: &WindowSpec) -> String {
    let mut parts = vec!["(".to_string()];
    if !window.partition_by.is_empty() {
        parts.push(format!("PARTITION BY {}", window.partition_by.join(", ")));
    }
    if !window.order_by.is_empty() {
        let ob: Vec<String> = window
            .order_by
            .iter()
            .map(|o| {
                if o.direction == "DESC" {
                    format!("{} DESC", o.column)
                } else {
                    format!("{} ASC", o.column)
                }
            })
            .collect();
        parts.push(format!("ORDER BY {}", ob.join(", ")));
    }
    if let Some(ref frame) = window.frame {
        let ft = match frame.frame_type {
            WindowFrameType::Rows => "ROWS",
            WindowFrameType::Range => "RANGE",
            WindowFrameType::Groups => "GROUPS",
        };
        let bound_str = |b: &FrameBound| -> String {
            match b {
                FrameBound::UnboundedPreceding => "UNBOUNDED PRECEDING".to_string(),
                FrameBound::UnboundedFollowing => "UNBOUNDED FOLLOWING".to_string(),
                FrameBound::CurrentRow => "CURRENT ROW".to_string(),
                FrameBound::NPreceding(n) => format!("{} PRECEDING", n),
                FrameBound::NFollowing(n) => format!("{} FOLLOWING", n),
            }
        };
        if let Some(ref end) = frame.end {
            parts.push(format!(
                "{} BETWEEN {} AND {}",
                ft,
                bound_str(&frame.start),
                bound_str(end)
            ));
        } else {
            parts.push(format!("{} {}", ft, bound_str(&frame.start)));
        }
    }
    format!("{})", parts.join(" "))
}

// ── Data Types ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedQuery {
    pub operation: QueryOperation,
    pub source_tables: Vec<String>,
    pub target_table: Option<String>,
    pub columns: Vec<String>,
    pub conditions: Option<String>,
    pub joins: Vec<JoinClause>,
    pub order_by: Vec<OrderByClause>,
    pub group_by: Vec<String>,
    pub aggregations: Vec<AggregationClause>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub set_operations: Vec<SetOperation>,
    pub nested_queries: Vec<Box<ParsedQuery>>,
    pub distinct: bool,
    pub having: Option<String>,
    pub ctes: Vec<CTEDefinition>,
    pub window_functions: Vec<WindowFunctionColumn>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryOperation {
    Select,
    Insert,
    Update,
    Delete,
    Create,
    Drop,
    Alter,
    Truncate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinClause {
    pub join_type: JoinType,
    pub table: String,
    pub condition: String,
    pub engine_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderByClause {
    pub column: String,
    pub direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationClause {
    pub agg_type: AggregationType,
    pub column: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationType {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    GroupConcat,
    ArrayAgg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CTEDefinition {
    pub name: String,
    pub columns: Option<Vec<String>>,
    pub query: Box<ParsedQuery>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowFunctionColumn {
    pub function: String,
    pub args: Vec<String>,
    pub window: WindowSpec,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSpec {
    pub partition_by: Vec<String>,
    pub order_by: Vec<OrderByClause>,
    pub frame: Option<WindowFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowFrame {
    pub frame_type: WindowFrameType,
    pub start: FrameBound,
    pub end: Option<FrameBound>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowFrameType {
    Rows,
    Range,
    Groups,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FrameBound {
    UnboundedPreceding,
    NPreceding(u64),
    CurrentRow,
    NFollowing(u64),
    UnboundedFollowing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetOperation {
    pub operation_type: SetOperationType,
    pub query: Box<ParsedQuery>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SetOperationType {
    Union,
    Intersect,
    Except,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_sql(sql: &str) -> ParsedQuery {
        let parser = QueryParser::new();
        let query = UqlQuery {
            query: sql.to_string(),
            query_type: QueryLanguage::Sql,
            parameters: None,
        };
        parser.parse(&query).unwrap()
    }

    #[test]
    fn test_simple_select() {
        let pq = parse_sql("SELECT * FROM users");
        assert_eq!(pq.operation, QueryOperation::Select);
        assert_eq!(pq.source_tables, vec!["users"]);
        assert_eq!(pq.columns, vec!["*"]);
    }

    #[test]
    fn test_select_columns() {
        let pq = parse_sql("SELECT id, name, email FROM users");
        assert_eq!(pq.columns, vec!["id", "name", "email"]);
        assert_eq!(pq.source_tables, vec!["users"]);
    }

    #[test]
    fn test_select_with_where() {
        let pq = parse_sql("SELECT * FROM users WHERE age > 25");
        assert!(pq.conditions.is_some());
        assert!(pq.conditions.unwrap().contains(">"));
    }

    #[test]
    fn test_select_with_and_where() {
        let pq = parse_sql("SELECT * FROM users WHERE age > 25 AND name = 'John'");
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("AND"));
    }

    #[test]
    fn test_select_with_order_by() {
        let pq = parse_sql("SELECT * FROM users ORDER BY name DESC");
        assert_eq!(pq.order_by.len(), 1);
        assert_eq!(pq.order_by[0].column, "name");
        assert_eq!(pq.order_by[0].direction, "DESC");
    }

    #[test]
    fn test_select_with_limit() {
        let pq = parse_sql("SELECT * FROM users LIMIT 10");
        assert_eq!(pq.limit, Some(10));
    }

    #[test]
    fn test_select_with_offset() {
        let pq = parse_sql("SELECT * FROM users LIMIT 10 OFFSET 20");
        assert_eq!(pq.limit, Some(10));
        assert_eq!(pq.offset, Some(20));
    }

    #[test]
    fn test_select_with_join() {
        let pq = parse_sql("SELECT * FROM users JOIN orders ON users.id = orders.user_id");
        assert_eq!(pq.joins.len(), 1);
        assert_eq!(pq.joins[0].table, "orders");
        assert_eq!(pq.joins[0].join_type, JoinType::Inner);
    }

    #[test]
    fn test_select_with_left_join() {
        let pq = parse_sql("SELECT * FROM users LEFT JOIN orders ON users.id = orders.user_id");
        assert_eq!(pq.joins.len(), 1);
        assert_eq!(pq.joins[0].join_type, JoinType::Left);
    }

    #[test]
    fn test_select_with_alias() {
        let pq = parse_sql("SELECT u.name FROM users AS u");
        assert_eq!(pq.source_tables, vec!["users"]);
    }

    #[test]
    fn test_insert() {
        let pq = parse_sql("INSERT INTO users (id, name) VALUES (1, 'Alice')");
        assert_eq!(pq.operation, QueryOperation::Insert);
        assert_eq!(pq.target_table, Some("users".to_string()));
    }

    #[test]
    fn test_update() {
        let pq = parse_sql("UPDATE users SET name = 'Bob' WHERE id = 1");
        assert_eq!(pq.operation, QueryOperation::Update);
        assert_eq!(pq.target_table, Some("users".to_string()));
        assert!(pq.conditions.is_some());
    }

    #[test]
    fn test_delete() {
        let pq = parse_sql("DELETE FROM users WHERE id = 1");
        assert_eq!(pq.operation, QueryOperation::Delete);
        assert_eq!(pq.target_table, Some("users".to_string()));
        assert!(pq.conditions.is_some());
    }

    #[test]
    fn test_create_table() {
        let pq = parse_sql("CREATE TABLE users (id INT, name TEXT)");
        assert_eq!(pq.operation, QueryOperation::Create);
        assert_eq!(pq.target_table, Some("users".to_string()));
    }

    #[test]
    fn test_drop_table() {
        let pq = parse_sql("DROP TABLE users");
        assert_eq!(pq.operation, QueryOperation::Drop);
        assert_eq!(pq.target_table, Some("users".to_string()));
    }

    #[test]
    fn test_quoted_identifier() {
        let pq = parse_sql(r#"SELECT "id", "name" FROM "users""#);
        assert_eq!(pq.source_tables, vec!["users"]);
        assert_eq!(pq.columns.len(), 2);
    }

    #[test]
    fn test_where_compound() {
        let pq = parse_sql("SELECT * FROM t WHERE (a > 1 OR b < 2) AND c = 3");
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("OR"));
        assert!(cond.contains("AND"));
    }

    #[test]
    fn test_is_null() {
        let pq = parse_sql("SELECT * FROM t WHERE name IS NULL");
        assert!(pq.conditions.unwrap().contains("IS NULL"));
    }

    #[test]
    fn test_is_not_null() {
        let pq = parse_sql("SELECT * FROM t WHERE name IS NOT NULL");
        assert!(pq.conditions.unwrap().contains("IS NOT NULL"));
    }

    #[test]
    fn test_in_clause() {
        let pq = parse_sql("SELECT * FROM t WHERE id IN (1, 2, 3)");
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("IN"));
    }

    #[test]
    fn test_like_clause() {
        let pq = parse_sql("SELECT * FROM t WHERE name LIKE '%John%'");
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("LIKE"));
    }

    #[test]
    fn test_between_clause() {
        let pq = parse_sql("SELECT * FROM t WHERE age BETWEEN 18 AND 65");
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("BETWEEN"));
    }

    #[test]
    fn test_group_by() {
        let pq = parse_sql("SELECT COUNT(*) FROM users GROUP BY status");
        assert_eq!(pq.group_by.len(), 1);
        assert_eq!(pq.group_by[0], "status");
    }

    #[test]
    fn test_aggregation() {
        let pq = parse_sql("SELECT COUNT(*) FROM users");
        assert!(pq.aggregations.len() > 0);
    }

    #[test]
    fn test_multiple_aggregations() {
        let pq = parse_sql("SELECT COUNT(*), AVG(age), MAX(salary) FROM users");
        assert_eq!(pq.aggregations.len(), 3);
    }

    #[test]
    fn test_tokenizer_basic() {
        let mut t = Tokenizer::new("SELECT * FROM users WHERE id = 1");
        let tokens = t.tokenize().unwrap();
        assert!(tokens.len() > 5);
        assert_eq!(tokens[0], Token::Select);
        assert_eq!(tokens[1], Token::Star);
        assert_eq!(tokens[2], Token::From);
    }

    #[test]
    fn test_tokenizer_string() {
        let mut t = Tokenizer::new("SELECT name FROM t WHERE name = 'hello world'");
        let tokens = t.tokenize().unwrap();
        assert!(tokens.contains(&Token::String("hello world".to_string())));
    }

    #[test]
    fn test_tokenizer_operators() {
        let mut t = Tokenizer::new("a = b AND c != d AND e >= f AND g <= h AND i <> j");
        let tokens = t.tokenize().unwrap();
        assert!(tokens.contains(&Token::Eq));
        assert!(tokens.contains(&Token::Neq));
        assert!(tokens.contains(&Token::Gte));
        assert!(tokens.contains(&Token::Lte));
    }

    #[test]
    fn test_alter_table_add_column() {
        let pq = parse_sql("ALTER TABLE users ADD COLUMN age INT");
        assert_eq!(pq.operation, QueryOperation::Alter);
        assert_eq!(pq.target_table, Some("users".to_string()));
    }

    #[test]
    fn test_alter_table_drop_column() {
        let pq = parse_sql("ALTER TABLE users DROP COLUMN age");
        assert_eq!(pq.target_table, Some("users".to_string()));
    }

    #[test]
    fn test_rename_table() {
        let pq = parse_sql("ALTER TABLE users RENAME TO customers");
        assert_eq!(pq.target_table, Some("users".to_string()));
        assert_eq!(pq.columns, vec!["customers"]);
    }

    #[test]
    fn test_create_index() {
        let pq = parse_sql("CREATE INDEX idx_name ON users");
        assert_eq!(pq.operation, QueryOperation::Create);
        assert_eq!(pq.target_table, Some("users".to_string()));
    }

    #[test]
    fn test_select_distinct() {
        let pq = parse_sql("SELECT DISTINCT name FROM users");
        assert!(pq.distinct);
    }

    #[test]
    fn test_cross_join() {
        let pq = parse_sql("SELECT * FROM users CROSS JOIN orders");
        assert_eq!(pq.joins.len(), 1);
        assert_eq!(pq.joins[0].join_type, JoinType::Cross);
    }

    #[test]
    fn test_where_or() {
        let pq = parse_sql("SELECT * FROM t WHERE a = 1 OR b = 2");
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("OR"));
    }

    #[test]
    fn test_not_expression() {
        let pq = parse_sql("SELECT * FROM t WHERE NOT active");
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("NOT"));
    }

    #[test]
    fn test_function_call() {
        let pq = parse_sql("SELECT LOWER(name) FROM users");
        // Functions remain in conditions string for now
        assert!(pq.columns.len() >= 1);
    }

    #[test]
    fn test_qualified_column() {
        let pq = parse_sql("SELECT users.id, users.name FROM users");
        assert_eq!(pq.columns, vec!["users.id", "users.name"]);
    }

    #[test]
    fn test_empty_input() {
        let result = parse_sql("SELECT * FROM empty_table");
        assert_eq!(result.source_tables, vec!["empty_table"]);
    }

    // ── MongoDB Parser Tests ──────────────────────────────────────

    fn parse_mongodb(query: &str) -> ParsedQuery {
        let parser = QueryParser::new();
        let uql_query = UqlQuery {
            query: query.to_string(),
            query_type: QueryLanguage::MongoDb,
            parameters: None,
        };
        parser.parse(&uql_query).unwrap()
    }

    #[test]
    fn test_mongodb_simple_eq() {
        let pq = parse_mongodb(r#"{"users": {"status": "active"}}"#);
        assert_eq!(pq.source_tables, vec!["users"]);
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("status"));
        assert!(cond.contains("="));
        assert!(cond.contains("active"));
    }

    #[test]
    fn test_mongodb_gt_lt() {
        let pq = parse_mongodb(r#"{"users": {"age": {"$gt": 25}}}"#);
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("age > 25"));
    }

    #[test]
    fn test_mongodb_gte_lte() {
        let pq = parse_mongodb(r#"{"users": {"age": {"$gte": 18, "$lte": 65}}}"#);
        let cond = pq.conditions.unwrap();
        assert!(cond.contains(">="));
        assert!(cond.contains("<="));
    }

    #[test]
    fn test_mongodb_in() {
        let pq = parse_mongodb(r#"{"users": {"status": {"$in": ["active", "pending"]}}}"#);
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("IN"));
        assert!(cond.contains("active"));
        assert!(cond.contains("pending"));
    }

    #[test]
    fn test_mongodb_and() {
        let pq = parse_mongodb(
            r#"{"$and": [{"users": {"age": {"$gt": 25}}}, {"users": {"status": "active"}}]}"#,
        );
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("AND"));
    }

    #[test]
    fn test_mongodb_or() {
        let pq = parse_mongodb(
            r#"{"$or": [{"users": {"status": "active"}}, {"users": {"age": {"$lt": 18}}}]}"#,
        );
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("OR"));
    }

    #[test]
    fn test_mongodb_exists() {
        let pq = parse_mongodb(r#"{"users": {"email": {"$exists": true}}}"#);
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("IS NOT NULL"));
    }

    #[test]
    fn test_mongodb_not_exists() {
        let pq = parse_mongodb(r#"{"users": {"email": {"$exists": false}}}"#);
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("IS NULL"));
    }

    #[test]
    fn test_mongodb_ne() {
        let pq = parse_mongodb(r#"{"users": {"status": {"$ne": "deleted"}}}"#);
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("!="));
    }

    #[test]
    fn test_mongodb_multiple_fields() {
        let pq = parse_mongodb(
            r#"{"users": {"age": {"$gt": 18}, "status": "active", "score": {"$gte": 50}}}"#,
        );
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("AND") || cond.contains("age"));
    }

    // ── Mango (CouchDB) Parser Tests ──────────────────────────────

    fn parse_mango(query: &str) -> ParsedQuery {
        let parser = QueryParser::new();
        let uql_query = UqlQuery {
            query: query.to_string(),
            query_type: QueryLanguage::Mango,
            parameters: None,
        };
        parser.parse(&uql_query).unwrap()
    }

    #[test]
    fn test_mango_simple_selector() {
        let pq = parse_mango(r#"{"selector": {"age": {"$gt": 25}}}"#);
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("age > 25"));
    }

    #[test]
    fn test_mango_with_limit() {
        let pq = parse_mango(r#"{"selector": {"status": "active"}, "limit": 10}"#);
        assert_eq!(pq.limit, Some(10));
    }

    #[test]
    fn test_mango_with_skip() {
        let pq = parse_mango(r#"{"selector": {}, "skip": 20}"#);
        assert_eq!(pq.offset, Some(20));
    }

    #[test]
    fn test_mango_with_sort() {
        let pq = parse_mango(r#"{"selector": {}, "sort": [{"age": "desc"}]}"#);
        assert_eq!(pq.order_by.len(), 1);
        assert_eq!(pq.order_by[0].column, "age");
        assert_eq!(pq.order_by[0].direction, "DESC");
    }

    #[test]
    fn test_mango_with_fields() {
        let pq = parse_mango(r#"{"selector": {}, "fields": ["id", "name"]}"#);
        assert_eq!(pq.columns, vec!["id", "name"]);
    }

    #[test]
    fn test_mango_mixed_operators() {
        let pq = parse_mango(
            r#"{"selector": {"age": {"$gte": 18, "$lte": 65}, "status": {"$in": ["active", "pending"]}}}"#,
        );
        let cond = pq.conditions.unwrap();
        assert!(cond.contains(">="));
        assert!(cond.contains("<="));
        assert!(cond.contains("IN"));
    }

    #[test]
    fn test_mango_and_operator() {
        let pq =
            parse_mango(r#"{"selector": {"$and": [{"age": {"$gt": 18}}, {"status": "active"}]}}"#);
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("AND"));
    }

    #[test]
    fn test_mango_or_operator() {
        let pq =
            parse_mango(r#"{"selector": {"$or": [{"status": "active"}, {"age": {"$lt": 18}}]}}"#);
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("OR"));
    }

    #[test]
    fn test_mango_regex() {
        let pq = parse_mango(r#"{"selector": {"name": {"$regex": "^John"}}}"#);
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("LIKE"));
    }

    #[test]
    fn test_mango_not_operator() {
        let pq = parse_mango(r#"{"selector": {"status": {"$not": {"$eq": "deleted"}}}}"#);
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("NOT"));
    }

    #[test]
    fn test_mango_nor_operator() {
        let pq =
            parse_mango(r#"{"selector": {"$nor": [{"age": {"$lt": 18}}, {"status": "banned"}]}}"#);
        let cond = pq.conditions.unwrap();
        assert!(cond.contains("NOT"));
    }

    #[test]
    fn test_auto_detect_mongodb_json() {
        let parser = QueryParser::new();
        let uql_query = UqlQuery {
            query: r#"{"users": {"age": {"$gt": 25}}}"#.to_string(),
            query_type: QueryLanguage::Auto,
            parameters: None,
        };
        let pq = parser.parse(&uql_query).unwrap();
        assert!(pq.conditions.is_some());
        assert_eq!(pq.source_tables, vec!["users"]);
    }

    #[test]
    fn test_mongodb_json_value_to_sql() {
        // Test json_value_to_sql_literal works for various types
        let s = QueryParser::json_value_to_sql_literal(&serde_json::json!("hello"));
        assert_eq!(s, "'hello'");

        let n = QueryParser::json_value_to_sql_literal(&serde_json::json!(42));
        assert_eq!(n, "42");

        let b = QueryParser::json_value_to_sql_literal(&serde_json::json!(true));
        assert_eq!(b, "TRUE");

        let arr = QueryParser::json_value_to_sql_literal(&serde_json::json!([1, 2, 3]));
        assert_eq!(arr, "(1, 2, 3)");
    }

    #[test]
    fn test_mango_empty_selector() {
        let pq = parse_mango(r#"{"selector": {}}"#);
        assert!(pq.conditions.is_none());
    }

    // ── CTE tests ──────────────────────────────────────────────

    #[test]
    fn test_simple_cte() {
        let pq = parse_sql("WITH cte AS (SELECT * FROM users) SELECT * FROM cte");
        assert_eq!(pq.ctes.len(), 1);
        assert_eq!(pq.ctes[0].name, "cte");
        assert_eq!(pq.ctes[0].columns, None);
    }

    #[test]
    fn test_cte_with_column_names() {
        let pq = parse_sql("WITH cte (id, name) AS (SELECT id, name FROM users) SELECT * FROM cte");
        assert_eq!(pq.ctes.len(), 1);
        assert_eq!(pq.ctes[0].name, "cte");
        assert_eq!(
            pq.ctes[0].columns,
            Some(vec!["id".to_string(), "name".to_string()])
        );
    }

    #[test]
    fn test_multiple_ctes() {
        let pq = parse_sql(
            "WITH a AS (SELECT * FROM t1), b AS (SELECT * FROM t2) SELECT * FROM a JOIN b ON a.id = b.id",
        );
        assert_eq!(pq.ctes.len(), 2);
        assert_eq!(pq.ctes[0].name, "a");
        assert_eq!(pq.ctes[1].name, "b");
    }

    #[test]
    fn test_cte_with_recursive() {
        let pq = parse_sql(
            "WITH RECURSIVE t(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM t WHERE n < 10) SELECT * FROM t",
        );
        assert_eq!(pq.ctes.len(), 1);
        assert_eq!(pq.ctes[0].name, "t");
    }

    #[test]
    fn test_cte_in_update() {
        let pq = parse_sql("WITH cte AS (SELECT id FROM users) UPDATE orders SET status = 'done' WHERE id IN (SELECT id FROM cte)");
        assert_eq!(pq.ctes.len(), 1);
        assert_eq!(pq.ctes[0].name, "cte");
    }

    // ── Window function tests ──────────────────────────────────

    #[test]
    fn test_row_number_over() {
        let pq = parse_sql("SELECT ROW_NUMBER() OVER (ORDER BY id) AS rn FROM users");
        assert_eq!(pq.window_functions.len(), 1);
        assert_eq!(pq.window_functions[0].function, "ROW_NUMBER");
        assert!(pq.window_functions[0].window.order_by.len() >= 1);
    }

    #[test]
    fn test_window_partition_by() {
        let pq =
            parse_sql("SELECT RANK() OVER (PARTITION BY dept ORDER BY salary DESC) FROM employees");
        assert_eq!(pq.window_functions.len(), 1);
        assert_eq!(pq.window_functions[0].function, "RANK");
        assert_eq!(pq.window_functions[0].window.partition_by, vec!["dept"]);
    }

    #[test]
    fn test_window_frame_rows() {
        let pq = parse_sql("SELECT SUM(amount) OVER (ORDER BY date ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING) FROM sales");
        assert_eq!(pq.window_functions.len(), 1);
        let frame = pq.window_functions[0].window.frame.as_ref().unwrap();
        assert!(matches!(frame.frame_type, WindowFrameType::Rows));
    }

    #[test]
    fn test_window_frame_range() {
        let pq = parse_sql("SELECT AVG(price) OVER (ORDER BY ts RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM ticks");
        assert_eq!(pq.window_functions.len(), 1);
        let frame = pq.window_functions[0].window.frame.as_ref().unwrap();
        assert!(matches!(frame.frame_type, WindowFrameType::Range));
    }

    #[test]
    fn test_window_frame_groups() {
        let pq = parse_sql("SELECT SUM(x) OVER (ORDER BY y GROUPS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM t");
        assert_eq!(pq.window_functions.len(), 1);
        let frame = pq.window_functions[0].window.frame.as_ref().unwrap();
        assert!(matches!(frame.frame_type, WindowFrameType::Groups));
    }

    #[test]
    fn test_multiple_window_functions() {
        let pq = parse_sql(
            "SELECT ROW_NUMBER() OVER (ORDER BY id) AS rn, RANK() OVER (ORDER BY score DESC) AS rk FROM results",
        );
        assert_eq!(pq.window_functions.len(), 2);
    }

    #[test]
    fn test_window_unbounded_preceding() {
        let pq = parse_sql("SELECT SUM(x) OVER (ORDER BY y ROWS UNBOUNDED PRECEDING) FROM t");
        assert_eq!(pq.window_functions.len(), 1);
        let frame = pq.window_functions[0].window.frame.as_ref().unwrap();
        assert!(matches!(frame.start, FrameBound::UnboundedPreceding));
    }

    #[test]
    fn test_window_current_row() {
        let pq = parse_sql("SELECT SUM(x) OVER (ORDER BY y ROWS CURRENT ROW) FROM t");
        assert_eq!(pq.window_functions.len(), 1);
        let frame = pq.window_functions[0].window.frame.as_ref().unwrap();
        assert!(matches!(frame.start, FrameBound::CurrentRow));
    }
}
