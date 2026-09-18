use thiserror::Error;

pub mod assembler;
pub use assembler::{AssembledProgram, Assembler, AssemblerError, AssemblerErrorKind};

pub mod symbol_table;
pub use symbol_table::{Symbol, SymbolTable, SymbolTableError};

pub mod disassembler;
pub use disassembler::{
    DecodedInstruction, Disassembler, DisassemblyOptions, InstructionKind, MnemonicCategory,
};

pub mod label;
pub use label::Label;

pub mod location;
pub use location::Location;

pub mod source_map;
pub use source_map::{FileId, SourceFile, SourceMap};

pub mod number;
pub use number::Number;

pub mod opcode;
pub use opcode::{OpCode, OpCodeModes};

pub mod parser;
pub use parser::{Parser, ParserError, ParserErrorKind};

pub mod preprocessor;
pub use preprocessor::{Preprocessor, PreprocessorError, PreprocessorErrorKind};

pub mod statement;
pub use statement::{ReferenceKind, Statement, StatementKind};

pub mod token;
pub use token::{Token, TokenKind};

pub mod tokenizer;
pub use tokenizer::Tokenizer;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum UxntalError {
    #[error(transparent)]
    Preprocessor(#[from] PreprocessorError),

    #[error(transparent)]
    Parser(#[from] ParserError),

    #[error(transparent)]
    Assembler(#[from] AssemblerError),
}

impl UxntalError {
    pub fn location(&self) -> Location {
        match self {
            Self::Preprocessor(err) => err.location(),
            Self::Parser(err) => err.location(),
            Self::Assembler(err) => err.location(),
        }
    }

    pub fn format_error(&self, source_map: &SourceMap) -> String {
        source_map.format_error(self.location(), &self.to_string())
    }

    pub fn format_error_with_source(&self, file_path: &str, source: &str) -> String {
        self.location()
            .format_error(file_path, source, &self.to_string())
    }
}
