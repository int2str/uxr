use thiserror::Error;

use super::location::Location;
use super::number::Number;
use super::opcode::OpCode;
use super::source_map::SourceMap;
use super::statement::{ReferenceKind, Statement, StatementKind};
use super::token::{Token, TokenKind};

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ParserErrorKind {
    #[error("A label with this name already exists")]
    LabelRedefinition,

    #[error("Invalid opcode")]
    InvalidOpCode,

    #[error("Invalid label name")]
    InvalidLabelName,

    #[error("Writing in zero-page")]
    WriteToZeroPage,

    #[error("Memory overwrite")]
    MemoryOverwrite,

    #[error("Invalid hex number")]
    InvalidHexLiteral,

    #[error("Unexpected token type - expected: {expected:?}, got {actual:?}")]
    UnexpectedToken {
        expected: TokenKind,
        actual: TokenKind,
    },

    #[error("Token type {kind:?} not implemented")]
    Unimplemented { kind: TokenKind },

    #[error("Unexpected end of input")]
    UnexpectedEof,
}

impl ParserErrorKind {
    pub fn at(self, location: Location) -> ParserError {
        ParserError {
            kind: self,
            location,
        }
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
#[error("{kind}")]
pub struct ParserError {
    kind: ParserErrorKind,
    location: Location,
}

impl ParserError {
    pub fn new(kind: ParserErrorKind, location: Location) -> Self {
        Self { kind, location }
    }

    pub fn location(&self) -> Location {
        self.location
    }

    pub fn kind(&self) -> &ParserErrorKind {
        &self.kind
    }

    pub fn format_error(&self, source_map: &SourceMap) -> String {
        source_map.format_error(self.location, &self.to_string())
    }
}

pub struct Parser<'t> {
    tokens: &'t [Token],
}

impl<'t> Parser<'t> {
    pub fn parse(tokens: &'t [Token]) -> Result<Vec<Statement>, ParserError> {
        let mut parser = Self::new(tokens);
        parser.parse_program()
    }

    pub fn new(tokens: &'t [Token]) -> Self {
        Self { tokens }
    }

    fn parse_program(&mut self) -> Result<Vec<Statement>, ParserError> {
        let mut statements = Vec::new();
        while let Some(token) = self.pop() {
            if matches!(token.kind, TokenKind::BracketOpen | TokenKind::BracketClose) {
                continue;
            }

            let statement = match token.kind {
                TokenKind::Number => self.parse_number(token)?,
                TokenKind::RawString => self.parse_raw_string(token)?,
                TokenKind::Identifier => self.parse_identifier(token)?,
                TokenKind::HexLiteral => self.parse_hex_literal(token)?,
                TokenKind::Label | TokenKind::Sublabel => self.parse_label(token)?,
                TokenKind::OpCode => self.parse_opcode(token)?,
                TokenKind::PadAbsolute | TokenKind::PadRelative => self.parse_padding(token)?,
                TokenKind::JumpImmediate
                | TokenKind::JumpConditional
                | TokenKind::LiteralAbsolute
                | TokenKind::LiteralRelative
                | TokenKind::LiteralZeroPage
                | TokenKind::RawAbsolute
                | TokenKind::RawRelative
                | TokenKind::RawZeroPage => self.parse_reference(token)?,
                _ => {
                    todo!("Unhandled token kind {:?}", token)
                }
            };
            statements.push(statement);
        }
        Ok(statements)
    }

    fn parse_opcode(&mut self, token: &Token) -> Result<Statement, ParserError> {
        let (opcode, opcode_modes) = OpCode::from_str_with_modes(&token.value)
            .ok_or_else(|| ParserErrorKind::InvalidOpCode.at(token.location))?;
        Ok(Statement::new(
            StatementKind::Instruction(opcode, opcode_modes),
            token.location,
        ))
    }

    fn parse_identifier(&mut self, token: &Token) -> Result<Statement, ParserError> {
        let is_valid = if token.value.starts_with('/') {
            is_valid_label_name(&token.value[1..], true)
        } else {
            is_valid_label_name(&token.value, false)
        };
        if !is_valid {
            return Err(ParserErrorKind::InvalidLabelName.at(token.location));
        }
        Ok(Statement::new(
            StatementKind::Reference {
                kind: ReferenceKind::JumpStash,
                target: token.value.to_string(),
            },
            token.location,
        ))
    }

    fn parse_raw_string(&mut self, token: &Token) -> Result<Statement, ParserError> {
        let bytes = token.value.as_bytes()[1..].to_vec();
        Ok(Statement::new(
            StatementKind::RawBytes(bytes),
            token.location,
        ))
    }

    fn parse_number(&mut self, token: &Token) -> Result<Statement, ParserError> {
        let number = Number::from_hex(&token.value)
            .ok_or_else(|| ParserErrorKind::InvalidHexLiteral.at(token.location))?;
        let bytes = match number {
            Number::Byte(value) => value.to_be_bytes().to_vec(),
            Number::Short(value) => value.to_be_bytes().to_vec(),
        };
        Ok(Statement::new(
            StatementKind::RawBytes(bytes),
            token.location,
        ))
    }

    fn parse_reference(&mut self, token: &Token) -> Result<Statement, ParserError> {
        let value = &token.value[1..];
        if !is_valid_reference_name(value) {
            return Err(ParserErrorKind::InvalidLabelName.at(token.location));
        }
        let kind = match token.kind {
            TokenKind::JumpImmediate => ReferenceKind::JumpImmediate,
            TokenKind::JumpConditional => ReferenceKind::JumpConditional,
            TokenKind::LiteralAbsolute => ReferenceKind::LiteralAbsolute,
            TokenKind::LiteralRelative => ReferenceKind::LiteralRelative,
            TokenKind::LiteralZeroPage => ReferenceKind::LiteralZeroPage,
            TokenKind::RawAbsolute => ReferenceKind::RawAbsolute,
            TokenKind::RawRelative => ReferenceKind::RawRelative,
            TokenKind::RawZeroPage => ReferenceKind::RawZeroPage,
            _ => unreachable!("Should only dispatch valid reference types"),
        };

        Ok(Statement::new(
            StatementKind::Reference {
                kind,
                target: value.to_string(),
            },
            token.location,
        ))
    }

    fn parse_padding(&mut self, token: &Token) -> Result<Statement, ParserError> {
        let number = Number::from_hex_padding(&token.value[1..])
            .ok_or_else(|| ParserErrorKind::InvalidHexLiteral.at(token.location))?;
        let kind = match token.kind {
            TokenKind::PadAbsolute => StatementKind::PadAbsolute(number),
            TokenKind::PadRelative => StatementKind::PadRelative(number),
            _ => unreachable!("Only these two ^^^ should be matched to get here"),
        };
        Ok(Statement::new(kind, token.location))
    }

    fn parse_label(&mut self, token: &Token) -> Result<Statement, ParserError> {
        let name = &token.value[1..];

        let is_sublabel = token.kind == TokenKind::Sublabel;
        if !is_valid_label_name(name, is_sublabel) {
            return Err(ParserErrorKind::InvalidLabelName.at(token.location));
        }

        let kind = match token.kind {
            TokenKind::Label => StatementKind::Label(name.to_string()),
            TokenKind::Sublabel => StatementKind::Sublabel(name.to_string()),
            _ => unreachable!("Only Label and Sublabel should match here"),
        };

        Ok(Statement::new(kind, token.location))
    }

    fn parse_hex_literal(&mut self, token: &Token) -> Result<Statement, ParserError> {
        match Number::from_hex(&token.value[1..]) {
            Some(Number::Byte(value)) => Ok(Statement::new(
                StatementKind::LiteralByte(value),
                token.location,
            )),
            Some(Number::Short(value)) => Ok(Statement::new(
                StatementKind::LiteralShort(value),
                token.location,
            )),
            None => Err(ParserErrorKind::InvalidHexLiteral.at(token.location)),
        }
    }

    fn pop(&mut self) -> Option<&'t Token> {
        let (token, remainder) = self.tokens.split_first()?;
        self.tokens = remainder;
        Some(token)
    }
}

fn is_rune(chr: char) -> bool {
    matches!(
        chr,
        '|' | '$'
            | '@'
            | '&'
            | '%'
            | '('
            | ','
            | '_'
            | '.'
            | '-'
            | ';'
            | '='
            | '?'
            | '!'
            | '#'
            | '}'
            | '~'
            | '['
            | ']'
            | '\"'
    )
}

fn is_valid_reference_name(name: &str) -> bool {
    if name.starts_with('&') || name.starts_with('/') {
        is_valid_label_name(&name[1..], true)
    } else {
        is_valid_label_name(name, false)
    }
}

fn is_valid_sublabel_name(name: &str) -> bool {
    !name.is_empty() && !name.contains('/') && !name.starts_with(is_rune)
}

fn is_valid_scope_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('/')
        && !name.starts_with(is_rune)
        && Number::from_hex(name).is_none()
        && OpCode::from_str_with_modes(name).is_none()
}

fn is_valid_label_name(name: &str, is_sublabel: bool) -> bool {
    if is_sublabel {
        is_valid_sublabel_name(name)
    } else if let Some((scope, sublabel)) = name.split_once('/') {
        is_valid_scope_name(scope) && is_valid_sublabel_name(sublabel)
    } else {
        is_valid_scope_name(name)
    }
}
