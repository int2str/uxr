use std::collections::HashMap;

use thiserror::Error;

use super::label::Label;
use super::location::Location;
use super::source_map::SourceMap;
use super::statement::{ReferenceKind, Statement, StatementKind};
use super::symbol_table::SymbolTable;

/// The output of assembling a Uxntal program, containing the ROM bytes and symbol table.
#[derive(Debug, Clone)]
pub struct AssembledProgram {
    pub rom: Vec<u8>,
    pub symbols: SymbolTable,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum AssemblerErrorKind {
    #[error("Label '{name}' not found")]
    UndefinedLabel { name: String },

    #[error("Writing in zero-page")]
    WriteToZeroPage,

    #[error("Memory overwrite")]
    MemoryOverwrite,

    #[error("ROM size exceeds 64KB capacity")]
    RomTooLarge,

    #[error("Label '{name}' already defined")]
    DuplicateLabel { name: String },

    #[error("Relative reference '{name}' is too far")]
    RelativeReferenceTooFar { name: String },
}

impl AssemblerErrorKind {
    pub fn at(self, location: Location) -> AssemblerError {
        AssemblerError {
            kind: self,
            location,
        }
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
#[error("{kind}")]
pub struct AssemblerError {
    kind: AssemblerErrorKind,
    location: Location,
}

impl AssemblerError {
    pub fn new(kind: AssemblerErrorKind, location: Location) -> Self {
        Self { kind, location }
    }

    pub fn location(&self) -> Location {
        self.location
    }

    pub fn kind(&self) -> &AssemblerErrorKind {
        &self.kind
    }

    pub fn format_error(&self, source_map: &SourceMap) -> String {
        source_map.format_error(self.location, &self.to_string())
    }
}

struct PendingReference {
    name: String,
    kind: ReferenceKind,
    addr: usize,
    location: Location,
}

pub struct Assembler {
    rom: [u8; 65_536],
    pointer: usize,
    size: usize,
}

impl Assembler {
    pub fn assemble(statements: &[Statement]) -> Result<Vec<u8>, AssemblerError> {
        let program = Self::assemble_with_symbols(statements)?;
        Ok(program.rom)
    }

    pub fn assemble_with_symbols(
        statements: &[Statement],
    ) -> Result<AssembledProgram, AssemblerError> {
        let mut assembler = Assembler::new();
        assembler.assemble_program(statements)
    }

    fn new() -> Self {
        Self {
            rom: [0; 65_536],
            pointer: 0x0100,
            size: 0x0100,
        }
    }

    fn write_byte(&mut self, byte: u8, location: Location) -> Result<(), AssemblerError> {
        if self.pointer < 0x0100 {
            return Err(AssemblerErrorKind::WriteToZeroPage.at(location));
        }
        if self.pointer >= 0x10000 {
            return Err(AssemblerErrorKind::RomTooLarge.at(location));
        }
        if self.pointer < self.size {
            return Err(AssemblerErrorKind::MemoryOverwrite.at(location));
        }
        self.rom[self.pointer] = byte;
        self.pointer += 1;
        self.size = self.pointer;
        Ok(())
    }

    fn write_bytes(&mut self, bytes: &[u8], location: Location) -> Result<(), AssemblerError> {
        for &byte in bytes {
            self.write_byte(byte, location)?;
        }
        Ok(())
    }

    fn assemble_program(
        &mut self,
        statements: &[Statement],
    ) -> Result<AssembledProgram, AssemblerError> {
        let mut scope = "on-reset".to_string();
        let mut labels: HashMap<String, Label> = HashMap::new();
        let mut symbols = SymbolTable::new();
        let mut pending_refs: Vec<PendingReference> = Vec::new();

        for statement in statements {
            match &statement.kind {
                StatementKind::PadAbsolute(addr) => {
                    self.pointer = *addr as usize;
                }
                StatementKind::PadRelative(offset) => {
                    self.pointer = self.pointer.saturating_add(*offset as usize);
                }
                StatementKind::Label(name) => {
                    if self.pointer >= 0x10000 {
                        return Err(AssemblerErrorKind::RomTooLarge.at(statement.location));
                    }
                    if labels.contains_key(name) {
                        return Err(AssemblerErrorKind::DuplicateLabel { name: name.clone() }
                            .at(statement.location));
                    }
                    labels.insert(name.clone(), Label::new(name, self.pointer as u16));
                    symbols.insert(name.clone(), self.pointer as u16);
                    if let Some((base, _)) = name.split_once('/') {
                        scope = base.to_string();
                    } else {
                        scope = name.clone();
                    }
                }
                StatementKind::Sublabel(sub) => {
                    if self.pointer >= 0x10000 {
                        return Err(AssemblerErrorKind::RomTooLarge.at(statement.location));
                    }
                    let full_name = format!("{scope}/{sub}");
                    if labels.contains_key(&full_name) {
                        return Err(AssemblerErrorKind::DuplicateLabel { name: full_name }
                            .at(statement.location));
                    }
                    labels.insert(
                        full_name.clone(),
                        Label::new(&full_name, self.pointer as u16),
                    );
                    symbols.insert(full_name.clone(), self.pointer as u16);
                }
                StatementKind::Instruction(opcode, modes) => {
                    let byte = opcode.as_u8() | modes.as_u8();
                    self.write_byte(byte, statement.location)?;
                }
                StatementKind::LiteralByte(byte) => {
                    self.write_byte(0x80, statement.location)?;
                    self.write_byte(*byte, statement.location)?;
                }
                StatementKind::LiteralShort(short) => {
                    self.write_byte(0xa0, statement.location)?;
                    self.write_byte((*short >> 8) as u8, statement.location)?;
                    self.write_byte((*short & 0xff) as u8, statement.location)?;
                }
                StatementKind::RawBytes(bytes) => {
                    self.write_bytes(bytes, statement.location)?;
                }
                StatementKind::Reference { kind, target } => {
                    let full_name = if target.starts_with('&') || target.starts_with('/') {
                        format!("{scope}/{}", &target[1..])
                    } else {
                        target.clone()
                    };
                    let addr = match kind {
                        ReferenceKind::LiteralAbsolute => {
                            self.write_byte(0xa0, statement.location)?;
                            let operand_addr = self.pointer;
                            self.write_byte(0x00, statement.location)?;
                            self.write_byte(0x00, statement.location)?;
                            operand_addr
                        }
                        ReferenceKind::LiteralRelative | ReferenceKind::LiteralZeroPage => {
                            self.write_byte(0x80, statement.location)?;
                            let operand_addr = self.pointer;
                            self.write_byte(0x00, statement.location)?;
                            operand_addr
                        }
                        ReferenceKind::RawAbsolute => {
                            let operand_addr = self.pointer;
                            self.write_byte(0x00, statement.location)?;
                            self.write_byte(0x00, statement.location)?;
                            operand_addr
                        }
                        ReferenceKind::RawRelative | ReferenceKind::RawZeroPage => {
                            let operand_addr = self.pointer;
                            self.write_byte(0x00, statement.location)?;
                            operand_addr
                        }
                        ReferenceKind::JumpImmediate => {
                            self.write_byte(0x40, statement.location)?;
                            let operand_addr = self.pointer;
                            self.write_byte(0x00, statement.location)?;
                            self.write_byte(0x00, statement.location)?;
                            operand_addr
                        }
                        ReferenceKind::JumpConditional => {
                            self.write_byte(0x20, statement.location)?;
                            let operand_addr = self.pointer;
                            self.write_byte(0x00, statement.location)?;
                            self.write_byte(0x00, statement.location)?;
                            operand_addr
                        }
                        ReferenceKind::JumpStash => {
                            self.write_byte(0x60, statement.location)?;
                            let operand_addr = self.pointer;
                            self.write_byte(0x00, statement.location)?;
                            self.write_byte(0x00, statement.location)?;
                            operand_addr
                        }
                    };
                    pending_refs.push(PendingReference {
                        name: full_name,
                        kind: *kind,
                        addr,
                        location: statement.location,
                    });
                }
            }
        }

        for ref_item in &pending_refs {
            let label = labels.get_mut(&ref_item.name).ok_or_else(|| {
                AssemblerErrorKind::UndefinedLabel {
                    name: ref_item.name.clone(),
                }
                .at(ref_item.location)
            })?;
            label.increment_references();
            let target_addr = label.address();

            match ref_item.kind {
                ReferenceKind::LiteralAbsolute | ReferenceKind::RawAbsolute => {
                    self.rom[ref_item.addr] = (target_addr >> 8) as u8;
                    self.rom[ref_item.addr + 1] = (target_addr & 0xff) as u8;
                }
                ReferenceKind::LiteralZeroPage | ReferenceKind::RawZeroPage => {
                    self.rom[ref_item.addr] = (target_addr & 0xff) as u8;
                }
                ReferenceKind::JumpImmediate
                | ReferenceKind::JumpConditional
                | ReferenceKind::JumpStash => {
                    let offset = target_addr.wrapping_sub((ref_item.addr + 2) as u16);
                    self.rom[ref_item.addr] = (offset >> 8) as u8;
                    self.rom[ref_item.addr + 1] = (offset & 0xff) as u8;
                }
                ReferenceKind::LiteralRelative | ReferenceKind::RawRelative => {
                    let diff = (target_addr as i32) - (ref_item.addr as i32) - 2;
                    if !(-128..=127).contains(&diff) {
                        return Err(AssemblerErrorKind::RelativeReferenceTooFar {
                            name: ref_item.name.clone(),
                        }
                        .at(ref_item.location));
                    }
                    self.rom[ref_item.addr] = diff as i8 as u8;
                }
            }
        }

        let rom = if self.size <= 0x0100 {
            Vec::new()
        } else {
            self.rom[0x0100..self.size].to_vec()
        };

        Ok(AssembledProgram { rom, symbols })
    }
}
