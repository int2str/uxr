use std::collections::HashMap;

use thiserror::Error;

use super::location::Location;
use super::source_map::SourceMap;
use super::token::{Token, TokenKind};
use super::tokenizer::Tokenizer;
use crate::shared::file_io::FileLoader;

const MAX_MACRO_RECURSION_DEPTH: usize = 32;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum PreprocessorErrorKind {
    #[error("Macro '{name}' already defined")]
    MacroRedefinition { name: String },

    #[error("Expected '{{' for macro definition")]
    ExpectedOpeningBrace,

    #[error("Expected a closing '}}'")]
    MissingClosingBrace,

    #[error("Unexpected closing '}}'")]
    UnexpectedClosingBrace,

    #[error("Macro recursion limit exceeded")]
    RecursionLimitExceeded,

    #[error("Failed to read include file '{path}': {message}")]
    FailedToReadFile { path: String, message: String },
}

impl PreprocessorErrorKind {
    pub fn at(self, location: Location) -> PreprocessorError {
        PreprocessorError {
            kind: self,
            location,
        }
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
#[error("{kind}")]
pub struct PreprocessorError {
    kind: PreprocessorErrorKind,
    location: Location,
}

impl PreprocessorError {
    pub fn new(kind: PreprocessorErrorKind, location: Location) -> Self {
        Self { kind, location }
    }

    pub fn location(&self) -> Location {
        self.location
    }

    pub fn kind(&self) -> &PreprocessorErrorKind {
        &self.kind
    }

    pub fn format_error(&self, source_map: &SourceMap) -> String {
        source_map.format_error(self.location, &self.to_string())
    }
}

pub struct Preprocessor<L> {
    file_loader: L,
    macros: HashMap<String, Vec<Token>>,
    scope: String,
    lambda_counter: usize,
}

impl<L: FileLoader> Preprocessor<L> {
    pub fn new(file_loader: L) -> Self {
        Self {
            file_loader,
            macros: HashMap::new(),
            scope: "on-reset".to_string(),
            lambda_counter: 0,
        }
    }

    pub fn macros(&self) -> &HashMap<String, Vec<Token>> {
        &self.macros
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }

    pub fn lambda_counter(&self) -> usize {
        self.lambda_counter
    }

    pub fn run(
        &mut self,
        source_map: &mut SourceMap,
        tokens: &[Token],
    ) -> Result<Vec<Token>, PreprocessorError> {
        let tokens_with_includes = self.parse_includes(source_map, tokens)?;
        let expanded_macros = self.expand_macros(&tokens_with_includes)?;
        self.expand_lambdas(&expanded_macros)
    }

    pub fn parse_includes(
        &mut self,
        source_map: &mut SourceMap,
        tokens: &[Token],
    ) -> Result<Vec<Token>, PreprocessorError> {
        let mut output = Vec::with_capacity(tokens.len());

        for token in tokens {
            if token.kind == TokenKind::Include {
                let path = &token.value[1..];
                let included_file = self.file_loader.read(path).map_err(|error| {
                    PreprocessorErrorKind::FailedToReadFile {
                        path: path.to_string(),
                        message: error.to_string(),
                    }
                    .at(token.location)
                })?;
                let included_tokens = Tokenizer::tokenize(source_map, path, &included_file);
                let recursively_included = self.parse_includes(source_map, &included_tokens)?;
                output.extend(recursively_included);
            } else {
                output.push(token.clone());
            }
        }

        Ok(output)
    }

    pub fn expand_macros(&mut self, tokens: &[Token]) -> Result<Vec<Token>, PreprocessorError> {
        let mut output = Vec::with_capacity(tokens.len());
        let mut index = 0;

        while index < tokens.len() {
            let token = &tokens[index];

            match token.kind {
                TokenKind::MacroDefinition => {
                    let macro_name = &token.value[1..];
                    if self.macros.contains_key(macro_name) {
                        return Err(PreprocessorErrorKind::MacroRedefinition {
                            name: macro_name.to_string(),
                        }
                        .at(token.location));
                    }

                    index += 1;
                    if index >= tokens.len() || tokens[index].kind != TokenKind::BraceOpen {
                        return Err(PreprocessorErrorKind::ExpectedOpeningBrace.at(token.location));
                    }

                    let open_brace_location = tokens[index].location;
                    index += 1;

                    let mut depth = 1usize;
                    let mut body_tokens = Vec::new();

                    while index < tokens.len() {
                        let current = &tokens[index];
                        index += 1;

                        match current.kind {
                            TokenKind::BraceClose => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                                body_tokens.push(current.clone());
                            }
                            _ if is_opening_lambda_token(current) => {
                                depth += 1;
                                body_tokens.push(current.clone());
                            }
                            _ => {
                                body_tokens.push(current.clone());
                            }
                        }
                    }

                    if depth != 0 {
                        return Err(
                            PreprocessorErrorKind::MissingClosingBrace.at(open_brace_location)
                        );
                    }

                    self.macros.insert(macro_name.to_string(), body_tokens);
                }
                TokenKind::Label => {
                    let label = &token.value[1..];
                    if let Some((scope, _)) = label.split_once('/') {
                        self.scope = scope.to_string();
                    } else {
                        self.scope = label.to_string();
                    }
                    output.push(token.clone());
                    index += 1;
                }
                _ => {
                    if let Some(macro_name) = self.resolve_macro_name(token) {
                        let inlined =
                            self.expand_macro_recursive(&macro_name, token.location, 1)?;
                        output.extend(inlined);
                    } else {
                        output.push(token.clone());
                    }
                    index += 1;
                }
            }
        }

        Ok(output)
    }

    fn resolve_macro_name(&self, token: &Token) -> Option<String> {
        if matches!(
            token.kind,
            TokenKind::MacroDefinition
                | TokenKind::Include
                | TokenKind::HexLiteral
                | TokenKind::Label
                | TokenKind::Sublabel
                | TokenKind::PadAbsolute
                | TokenKind::PadRelative
                | TokenKind::LiteralAbsolute
                | TokenKind::LiteralRelative
                | TokenKind::LiteralZeroPage
                | TokenKind::RawAbsolute
                | TokenKind::RawRelative
                | TokenKind::RawZeroPage
                | TokenKind::RawString
                | TokenKind::JumpImmediate
                | TokenKind::JumpConditional
                | TokenKind::BracketOpen
                | TokenKind::BracketClose
                | TokenKind::BraceOpen
                | TokenKind::BraceClose
        ) {
            return None;
        }

        if token.value.starts_with('/') {
            let scoped_name = format!("{}/{}", self.scope, &token.value[1..]);
            if self.macros.contains_key(&scoped_name) {
                return Some(scoped_name);
            }
        }

        if self.macros.contains_key(&token.value) {
            return Some(token.value.clone());
        }

        None
    }

    fn expand_macro_recursive(
        &self,
        macro_name: &str,
        call_location: Location,
        depth: usize,
    ) -> Result<Vec<Token>, PreprocessorError> {
        if depth > MAX_MACRO_RECURSION_DEPTH {
            return Err(PreprocessorErrorKind::RecursionLimitExceeded.at(call_location));
        }

        let body = self
            .macros
            .get(macro_name)
            .expect("macro must exist if resolved");

        let mut expanded = Vec::with_capacity(body.len());

        for token in body {
            if let Some(nested_macro) = self.resolve_macro_name(token) {
                let nested_expanded =
                    self.expand_macro_recursive(&nested_macro, token.location, depth + 1)?;
                expanded.extend(nested_expanded);
            } else {
                expanded.push(token.clone());
            }
        }

        Ok(expanded)
    }

    /// Expands anonymous label blocks (lambdas) such as `?{ ... }`, `!{ ... }`,
    /// `;{ ... }`, `_{ ... }`, and `{ ... }` into synthetic unique sublabel
    /// references and matching sublabel definitions.
    ///
    /// Labels are named `anon_1__`, `anon_2__`, etc. to avoid colliding with runes
    /// or user symbols.
    pub fn expand_lambdas(&mut self, tokens: &[Token]) -> Result<Vec<Token>, PreprocessorError> {
        let mut output = Vec::with_capacity(tokens.len());
        let mut stack: Vec<(String, Location)> = Vec::new();

        for token in tokens {
            if is_opening_lambda_token(token) {
                self.lambda_counter += 1;
                let label_name = format!("anon_{}__", self.lambda_counter);
                stack.push((label_name.clone(), token.location));

                output.push(transform_opening_lambda(token, &label_name));
            } else if token.kind == TokenKind::BraceClose {
                let (label_name, _open_location) = stack.pop().ok_or_else(|| {
                    PreprocessorErrorKind::UnexpectedClosingBrace.at(token.location)
                })?;

                output.push(Token {
                    kind: TokenKind::Sublabel,
                    value: format!("&{}", label_name),
                    location: token.location,
                });
            } else {
                output.push(token.clone());
            }
        }

        if let Some((_label_name, open_location)) = stack.pop() {
            return Err(PreprocessorErrorKind::MissingClosingBrace.at(open_location));
        }

        Ok(output)
    }

    /// Alias for [`expand_lambdas`].
    pub fn expand_anonymous_labels(
        &mut self,
        tokens: &[Token],
    ) -> Result<Vec<Token>, PreprocessorError> {
        self.expand_lambdas(tokens)
    }

    /// Convenience function to preprocess tokens using the provided file loader
    pub fn preprocess(
        source_map: &mut SourceMap,
        tokens: &[Token],
        file_loader: L,
    ) -> Result<Vec<Token>, PreprocessorError> {
        let mut preprocessor = Self::new(file_loader);
        preprocessor.run(source_map, tokens)
    }
}

fn is_opening_lambda_token(token: &Token) -> bool {
    token.kind == TokenKind::BraceOpen
        || (token.value.ends_with('{')
            && matches!(
                token.kind,
                TokenKind::JumpConditional
                    | TokenKind::JumpImmediate
                    | TokenKind::LiteralAbsolute
                    | TokenKind::LiteralRelative
                    | TokenKind::LiteralZeroPage
                    | TokenKind::RawAbsolute
                    | TokenKind::RawZeroPage
                    | TokenKind::RawRelative
            ))
}

fn transform_opening_lambda(token: &Token, label_name: &str) -> Token {
    match token.kind {
        TokenKind::BraceOpen => Token {
            kind: TokenKind::Identifier,
            value: format!("/{}", label_name),
            location: token.location,
        },
        TokenKind::JumpConditional => Token {
            kind: TokenKind::JumpConditional,
            value: format!("?&{}", label_name),
            location: token.location,
        },
        TokenKind::JumpImmediate => Token {
            kind: TokenKind::JumpImmediate,
            value: format!("!&{}", label_name),
            location: token.location,
        },
        TokenKind::LiteralAbsolute => Token {
            kind: TokenKind::LiteralAbsolute,
            value: format!(";&{}", label_name),
            location: token.location,
        },
        TokenKind::LiteralRelative => Token {
            kind: TokenKind::LiteralRelative,
            value: format!(",&{}", label_name),
            location: token.location,
        },
        TokenKind::LiteralZeroPage => Token {
            kind: TokenKind::LiteralZeroPage,
            value: format!(".&{}", label_name),
            location: token.location,
        },
        TokenKind::RawAbsolute => Token {
            kind: TokenKind::RawAbsolute,
            value: format!("=&{}", label_name),
            location: token.location,
        },
        TokenKind::RawZeroPage => Token {
            kind: TokenKind::RawZeroPage,
            value: format!("-&{}", label_name),
            location: token.location,
        },
        TokenKind::RawRelative => Token {
            kind: TokenKind::RawRelative,
            value: format!("_&{}", label_name),
            location: token.location,
        },
        _ => {
            let prefix = token.value.strip_suffix('{').unwrap_or(&token.value);
            Token {
                kind: token.kind,
                value: format!("{}&{}", prefix, label_name),
                location: token.location,
            }
        }
    }
}
