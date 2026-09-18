use super::location::Location;
use super::source_map::{FileId, SourceMap};
use super::token::{Token, TokenKind};

pub struct Tokenizer<'a> {
    source: &'a str,
    cursor: Location,
}

impl<'a> Tokenizer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self::with_file(FileId::default(), source)
    }

    pub fn with_file(file_id: FileId, source: &'a str) -> Self {
        Self {
            source,
            cursor: Location {
                file_id,
                offset: 0,
                line: 1,
                column: 1,
            },
        }
    }

    /// Registers the file with the source map and returns its tokens.
    /// This is the primary entry point for tokenizing files in the assembly pipeline.
    pub fn tokenize(source_map: &mut SourceMap, file_path: &str, source: &str) -> Vec<Token> {
        let file_id = source_map.add_file(file_path, source);
        Tokenizer::with_file(file_id, source).collect()
    }

    /// Helper to tokenize a string without an explicit source map (uses `FileId::default()`).
    pub fn tokenize_str(source: &str) -> Vec<Token> {
        Tokenizer::new(source).collect()
    }

    fn skip_whitespace(&mut self) -> bool {
        let skipped = self.pop_while(|chr| chr.is_ascii_whitespace());
        !skipped.is_empty()
    }

    fn skip_comment(&mut self) -> bool {
        if self.peek() != Some('(') {
            return false;
        }

        let mut depth = 0;

        while let Some(chr) = self.pop() {
            if chr == '(' {
                depth += 1;
            } else if chr == ')' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
        }

        true
    }

    fn parse_number_or_identifier(&mut self) -> TokenKind {
        let word = self.pop_while(|chr| !chr.is_ascii_whitespace());

        let is_hex = (word.len() == 2 || word.len() == 4) && word.chars().all(is_nibble);
        if is_hex {
            TokenKind::Number
        } else {
            TokenKind::Identifier
        }
    }

    fn pop_while(&mut self, mut predicate: impl FnMut(char) -> bool) -> &'a str {
        let start = self.cursor;
        while self.peek().is_some_and(&mut predicate) {
            self.pop();
        }
        &self.source[start.offset..self.cursor.offset]
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.cursor.offset..)?.chars().next()
    }

    fn pop(&mut self) -> Option<char> {
        let chr = self.peek()?;
        self.cursor.offset += chr.len_utf8();
        match chr {
            '\n' => {
                self.cursor.line += 1;
                self.cursor.column = 1;
            }
            _ => self.cursor.column += 1,
        }
        Some(chr)
    }

    fn pop_and<T>(&mut self, value: T) -> T {
        self.pop();
        value
    }

    fn pop_word_and<T>(&mut self, value: T) -> T {
        self.pop();
        self.pop_while(|chr| !chr.is_ascii_whitespace());
        value
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        while self.skip_whitespace() || self.skip_comment() {}

        if self.cursor.offset == self.source.len() {
            return None;
        }

        let token_start = self.cursor;

        let kind = match self.peek() {
            Some('{') => self.pop_and(TokenKind::BraceOpen),
            Some('}') => self.pop_and(TokenKind::BraceClose),
            Some('[') => self.pop_and(TokenKind::BracketOpen),
            Some(']') => self.pop_and(TokenKind::BracketClose),
            Some('~') => self.pop_word_and(TokenKind::Include),
            Some('%') => self.pop_word_and(TokenKind::MacroDefinition),
            Some('|') => self.pop_word_and(TokenKind::PadAbsolute),
            Some('$') => self.pop_word_and(TokenKind::PadRelative),
            Some('@') => self.pop_word_and(TokenKind::Label),
            Some('&') => self.pop_word_and(TokenKind::Sublabel),
            Some('#') => self.pop_word_and(TokenKind::HexLiteral),
            Some('!') => self.pop_word_and(TokenKind::JumpImmediate),
            Some('?') => self.pop_word_and(TokenKind::JumpConditional),
            Some(';') => self.pop_word_and(TokenKind::LiteralAbsolute),
            Some('.') => self.pop_word_and(TokenKind::LiteralZeroPage),
            Some(',') => self.pop_word_and(TokenKind::LiteralRelative),
            Some('=') => self.pop_word_and(TokenKind::RawAbsolute),
            Some('-') => self.pop_word_and(TokenKind::RawZeroPage),
            Some('_') => self.pop_word_and(TokenKind::RawRelative),
            Some('"') => self.pop_word_and(TokenKind::RawString),
            Some('A'..='Z') => self.pop_word_and(TokenKind::OpCode),
            _ => self.parse_number_or_identifier(),
        };

        Some(Token {
            kind,
            value: self.source[token_start.offset..self.cursor.offset].to_string(),
            location: token_start,
        })
    }
}

fn is_nibble(chr: char) -> bool {
    matches!(chr, '0'..='9' | 'a'..='f')
}
