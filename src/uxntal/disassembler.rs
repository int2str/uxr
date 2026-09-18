use std::fmt::Write;

use super::opcode::{OpCode, OpCodeModes};
use super::symbol_table::SymbolTable;

/// Category of opcode mnemonic for classification and syntax highlighting.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MnemonicCategory {
    ControlFlow,
    Literal,
    Arithmetic,
    Comparison,
    MemoryOrStack,
    Raw,
}

/// Configuration options for disassembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisassemblyOptions {
    pub base_address: u16,
    pub disassemble_zeroes: bool,
}

impl Default for DisassemblyOptions {
    fn default() -> Self {
        Self {
            base_address: 0x0100,
            disassemble_zeroes: false,
        }
    }
}

/// The kind of instruction decoded from bytecode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructionKind {
    Brk,
    Jci { offset: i16, target_address: u16 },
    Jmi { offset: i16, target_address: u16 },
    Jsi { offset: i16, target_address: u16 },
    Lit { value: u8 },
    Lit2 { value: u16 },
    Litr { value: u8 },
    Lit2r { value: u16 },
    Regular { opcode: OpCode, modes: OpCodeModes },
    RawByte(u8),
}

/// A fully decoded instruction with its location, raw bytes, and representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedInstruction {
    pub address: u16,
    pub bytes: Vec<u8>,
    pub kind: InstructionKind,
    pub mnemonic: String,
    pub category: MnemonicCategory,
}

impl DecodedInstruction {
    pub fn byte_len(&self) -> usize {
        self.bytes.len()
    }
}

/// Disassembles Uxn bytecode into objdump-like text.
pub struct Disassembler;

impl Disassembler {
    /// Decodes a single instruction at the given offset within the ROM slice.
    pub fn decode_instruction(
        rom: &[u8],
        offset: usize,
        address: u16,
    ) -> Option<DecodedInstruction> {
        if offset >= rom.len() {
            return None;
        }

        let first_byte = rom[offset];

        // Immediate opcodes and BRK (base 0x00)
        if (first_byte & 0x1f) == 0 {
            match first_byte {
                0x00 => Some(DecodedInstruction {
                    address,
                    bytes: vec![first_byte],
                    kind: InstructionKind::Brk,
                    mnemonic: "BRK".to_string(),
                    category: MnemonicCategory::ControlFlow,
                }),
                0x20 => Self::decode_jump_immediate(
                    rom,
                    offset,
                    address,
                    first_byte,
                    "JCI",
                    |offset_value, target_address| InstructionKind::Jci {
                        offset: offset_value,
                        target_address,
                    },
                ),
                0x40 => Self::decode_jump_immediate(
                    rom,
                    offset,
                    address,
                    first_byte,
                    "JMI",
                    |offset_value, target_address| InstructionKind::Jmi {
                        offset: offset_value,
                        target_address,
                    },
                ),
                0x60 => Self::decode_jump_immediate(
                    rom,
                    offset,
                    address,
                    first_byte,
                    "JSI",
                    |offset_value, target_address| InstructionKind::Jsi {
                        offset: offset_value,
                        target_address,
                    },
                ),
                0x80 => {
                    if offset + 1 < rom.len() {
                        let value = rom[offset + 1];
                        Some(DecodedInstruction {
                            address,
                            bytes: vec![first_byte, value],
                            kind: InstructionKind::Lit { value },
                            mnemonic: "LIT".to_string(),
                            category: MnemonicCategory::Literal,
                        })
                    } else {
                        Some(Self::raw_byte_instruction(address, first_byte))
                    }
                }
                0xa0 => {
                    if offset + 2 < rom.len() {
                        let high = rom[offset + 1];
                        let low = rom[offset + 2];
                        let value = ((high as u16) << 8) | (low as u16);
                        Some(DecodedInstruction {
                            address,
                            bytes: vec![first_byte, high, low],
                            kind: InstructionKind::Lit2 { value },
                            mnemonic: "LIT2".to_string(),
                            category: MnemonicCategory::Literal,
                        })
                    } else {
                        Some(Self::raw_byte_instruction(address, first_byte))
                    }
                }
                0xc0 => {
                    if offset + 1 < rom.len() {
                        let value = rom[offset + 1];
                        Some(DecodedInstruction {
                            address,
                            bytes: vec![first_byte, value],
                            kind: InstructionKind::Litr { value },
                            mnemonic: "LITr".to_string(),
                            category: MnemonicCategory::Literal,
                        })
                    } else {
                        Some(Self::raw_byte_instruction(address, first_byte))
                    }
                }
                0xe0 => {
                    if offset + 2 < rom.len() {
                        let high = rom[offset + 1];
                        let low = rom[offset + 2];
                        let value = ((high as u16) << 8) | (low as u16);
                        Some(DecodedInstruction {
                            address,
                            bytes: vec![first_byte, high, low],
                            kind: InstructionKind::Lit2r { value },
                            mnemonic: "LIT2r".to_string(),
                            category: MnemonicCategory::Literal,
                        })
                    } else {
                        Some(Self::raw_byte_instruction(address, first_byte))
                    }
                }
                _ => Some(Self::raw_byte_instruction(address, first_byte)),
            }
        } else {
            // Standard instructions (base 0x01..=0x1f)
            let base_opcode_number = first_byte & 0x1f;
            let modes = OpCodeModes::from(first_byte & 0xe0);

            let (base_mnemonic, category) = match base_opcode_number {
                0x01 => ("INC", MnemonicCategory::Arithmetic),
                0x02 => ("POP", MnemonicCategory::MemoryOrStack),
                0x03 => ("NIP", MnemonicCategory::MemoryOrStack),
                0x04 => ("SWP", MnemonicCategory::MemoryOrStack),
                0x05 => ("ROT", MnemonicCategory::MemoryOrStack),
                0x06 => ("DUP", MnemonicCategory::MemoryOrStack),
                0x07 => ("OVR", MnemonicCategory::MemoryOrStack),
                0x08 => ("EQU", MnemonicCategory::Comparison),
                0x09 => ("NEQ", MnemonicCategory::Comparison),
                0x0a => ("GTH", MnemonicCategory::Comparison),
                0x0b => ("LTH", MnemonicCategory::Comparison),
                0x0c => ("JMP", MnemonicCategory::ControlFlow),
                0x0d => ("JCN", MnemonicCategory::ControlFlow),
                0x0e => ("JSR", MnemonicCategory::ControlFlow),
                0x0f => ("STH", MnemonicCategory::MemoryOrStack),
                0x10 => ("LDZ", MnemonicCategory::MemoryOrStack),
                0x11 => ("STZ", MnemonicCategory::MemoryOrStack),
                0x12 => ("LDR", MnemonicCategory::MemoryOrStack),
                0x13 => ("STR", MnemonicCategory::MemoryOrStack),
                0x14 => ("LDA", MnemonicCategory::MemoryOrStack),
                0x15 => ("STA", MnemonicCategory::MemoryOrStack),
                0x16 => ("DEI", MnemonicCategory::MemoryOrStack),
                0x17 => ("DEO", MnemonicCategory::MemoryOrStack),
                0x18 => ("ADD", MnemonicCategory::Arithmetic),
                0x19 => ("SUB", MnemonicCategory::Arithmetic),
                0x1a => ("MUL", MnemonicCategory::Arithmetic),
                0x1b => ("DIV", MnemonicCategory::Arithmetic),
                0x1c => ("AND", MnemonicCategory::Arithmetic),
                0x1d => ("ORA", MnemonicCategory::Arithmetic),
                0x1e => ("EOR", MnemonicCategory::Arithmetic),
                0x1f => ("SFT", MnemonicCategory::Arithmetic),
                _ => unreachable!(),
            };

            let opcode = OpCode::from_mnemonic(base_mnemonic).unwrap_or(OpCode::BRK);

            let mut mnemonic = base_mnemonic.to_string();
            if modes.is_short() {
                mnemonic.push('2');
            }
            if modes.is_return() {
                mnemonic.push('r');
            }
            if modes.is_keep() {
                mnemonic.push('k');
            }

            Some(DecodedInstruction {
                address,
                bytes: vec![first_byte],
                kind: InstructionKind::Regular { opcode, modes },
                mnemonic,
                category,
            })
        }
    }

    fn decode_jump_immediate<F>(
        rom: &[u8],
        offset: usize,
        address: u16,
        first_byte: u8,
        mnemonic: &str,
        kind_constructor: F,
    ) -> Option<DecodedInstruction>
    where
        F: FnOnce(i16, u16) -> InstructionKind,
    {
        if offset + 2 < rom.len() {
            let high = rom[offset + 1];
            let low = rom[offset + 2];
            let offset_value = (((high as u16) << 8) | (low as u16)) as i16;
            // Target address in Uxn immediate jumps is relative to the address following the 3-byte instruction
            let next_instruction_address = address.wrapping_add(3);
            let target_address = next_instruction_address.wrapping_add(offset_value as u16);

            Some(DecodedInstruction {
                address,
                bytes: vec![first_byte, high, low],
                kind: kind_constructor(offset_value, target_address),
                mnemonic: mnemonic.to_string(),
                category: MnemonicCategory::ControlFlow,
            })
        } else {
            Some(Self::raw_byte_instruction(address, first_byte))
        }
    }

    fn raw_byte_instruction(address: u16, byte: u8) -> DecodedInstruction {
        DecodedInstruction {
            address,
            bytes: vec![byte],
            kind: InstructionKind::RawByte(byte),
            mnemonic: format!("#{byte:02x}"),
            category: MnemonicCategory::Raw,
        }
    }

    /// Disassembles the full ROM binary and formats it in objdump style.
    pub fn disassemble(
        rom: &[u8],
        symbol_table: Option<&SymbolTable>,
        options: &DisassemblyOptions,
    ) -> String {
        let mut output = String::new();

        let mut offset = 0;
        let mut consecutive_zero_count = 0;

        while offset < rom.len() {
            let address = options.base_address.wrapping_add(offset as u16);

            let has_symbol = symbol_table
                .map(|table| !table.lookup_all_at_address(address).is_empty())
                .unwrap_or(false);

            // Check for zero-suppression (runs of unlabelled 0x00 BRK instructions)
            if !options.disassemble_zeroes && !has_symbol && rom[offset] == 0x00 {
                consecutive_zero_count += 1;
                if consecutive_zero_count == 4 {
                    output.push_str("...\n");
                    offset += 1;
                    continue;
                } else if consecutive_zero_count > 4 {
                    offset += 1;
                    continue;
                }
            } else {
                consecutive_zero_count = 0;
            }

            // Print symbol header if this address has one or more labels
            if let Some(table) = symbol_table {
                let symbols_here = table.lookup_all_at_address(address);
                if !symbols_here.is_empty() {
                    if !output.is_empty() && !output.ends_with("\n\n") {
                        output.push('\n');
                    }
                    for symbol in symbols_here {
                        let _ = writeln!(output, "{:08x} <{}>:", address, symbol.name());
                    }
                }
            }

            if let Some(instruction) = Self::decode_instruction(rom, offset, address) {
                let formatted_line = Self::format_instruction_line(&instruction, symbol_table);
                output.push_str(&formatted_line);
                output.push('\n');
                offset += instruction.byte_len();
            } else {
                break;
            }
        }

        output
    }

    /// Formats a single instruction line with columns: address, raw bytes, mnemonic, operands.
    fn format_instruction_line(
        instruction: &DecodedInstruction,
        symbol_table: Option<&SymbolTable>,
    ) -> String {
        let mut line = String::new();

        // 1. Address column: "  0100:"
        let address_plain = format!("  {:04x}:", instruction.address);
        let _ = write!(line, "{address_plain}\t");

        // 2. Raw bytes column: "80 12         " (padded to 10 chars)
        let mut raw_bytes_string = String::new();
        for (byte_index, byte) in instruction.bytes.iter().enumerate() {
            if byte_index > 0 {
                raw_bytes_string.push(' ');
            }
            let _ = write!(raw_bytes_string, "{byte:02x}");
        }
        let raw_bytes_padded = format!("{raw_bytes_string:<10}");
        let _ = write!(line, "{raw_bytes_padded}\t");

        // 3. Mnemonic column: "LIT2    " (padded to 8 chars)
        let _ = write!(line, "{:<8}", instruction.mnemonic);

        // 4. Operands & Symbol Annotations
        match &instruction.kind {
            InstructionKind::Jci { target_address, .. }
            | InstructionKind::Jmi { target_address, .. }
            | InstructionKind::Jsi { target_address, .. } => {
                let target_hex = format!("{:04x}", target_address);
                let _ = write!(line, "{target_hex}");

                if let Some(annotation) =
                    Self::resolve_symbol_annotation(*target_address, symbol_table)
                {
                    let _ = write!(line, " {annotation}");
                }
            }
            InstructionKind::Lit { value } | InstructionKind::Litr { value } => {
                let _ = write!(line, "#{value:02x}");

                // If this byte literal matches a zero-page device symbol, annotate it
                if let Some(annotation) = Self::resolve_exact_symbol(*value as u16, symbol_table) {
                    let _ = write!(line, " {annotation}");
                }
            }
            InstructionKind::Lit2 { value } | InstructionKind::Lit2r { value } => {
                let _ = write!(line, "#{value:04x}");

                if let Some(annotation) = Self::resolve_symbol_annotation(*value, symbol_table) {
                    let _ = write!(line, " {annotation}");
                }
            }
            InstructionKind::Brk
            | InstructionKind::Regular { .. }
            | InstructionKind::RawByte(_) => {
                // No additional operands
            }
        }

        line
    }

    /// Resolves an exact symbol name at the given address, formatted as `<name>`.
    fn resolve_exact_symbol(address: u16, symbol_table: Option<&SymbolTable>) -> Option<String> {
        let table = symbol_table?;
        let symbol = table.lookup_address(address)?;
        Some(format!("<{}>", symbol.name()))
    }

    /// Resolves a symbol annotation for an address: `<name>` or `<name+0xoffset>`.
    fn resolve_symbol_annotation(
        address: u16,
        symbol_table: Option<&SymbolTable>,
    ) -> Option<String> {
        let table = symbol_table?;

        if let Some(symbol) = table.lookup_address(address) {
            return Some(format!("<{}>", symbol.name()));
        }

        // Check nearest preceding symbol
        if let Some((symbol, offset)) = table.find_nearest_symbol(address) {
            // Only annotate if within a reasonable distance (e.g. within 256 bytes of the symbol)
            if offset > 0 && offset <= 0x0100 {
                return Some(format!("<{}+0x{offset:x}>", symbol.name()));
            }
        }

        None
    }
}
