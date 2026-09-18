use std::collections::HashMap;

use uxr::uxntal::{DecodedInstruction, Disassembler, InstructionKind, SymbolTable};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisassemblyLineKind {
    Label {
        name: String,
        address: u16,
    },
    Instruction {
        instruction: DecodedInstruction,
        target_symbol: Option<String>,
    },
    OmittedZeroes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisassemblyLine {
    pub kind: DisassemblyLineKind,
    pub address: Option<u16>,
}

pub struct DisassemblyView {
    lines: Vec<DisassemblyLine>,
    address_to_line_index: HashMap<u16, usize>,
}

impl DisassemblyView {
    pub fn new(rom: &[u8], symbol_table: Option<&SymbolTable>, base_address: u16) -> Self {
        let mut lines = Vec::new();
        let mut address_to_line_index = HashMap::new();

        let mut offset = 0;
        let mut consecutive_zero_count = 0;

        while offset < rom.len() {
            let address = base_address.wrapping_add(offset as u16);

            let has_symbol = symbol_table
                .map(|table| !table.lookup_all_at_address(address).is_empty())
                .unwrap_or(false);

            if !has_symbol && rom[offset] == 0x00 {
                consecutive_zero_count += 1;
                if consecutive_zero_count == 4 {
                    lines.push(DisassemblyLine {
                        kind: DisassemblyLineKind::OmittedZeroes,
                        address: None,
                    });
                    offset += 1;
                    continue;
                } else if consecutive_zero_count > 4 {
                    offset += 1;
                    continue;
                }
            } else {
                consecutive_zero_count = 0;
            }

            if let Some(table) = symbol_table {
                let symbols = table.lookup_all_at_address(address);
                for symbol in symbols {
                    lines.push(DisassemblyLine {
                        kind: DisassemblyLineKind::Label {
                            name: symbol.name().to_string(),
                            address,
                        },
                        address: Some(address),
                    });
                }
            }

            if let Some(instruction) = Disassembler::decode_instruction(rom, offset, address) {
                let target_symbol = Self::resolve_target_symbol(&instruction, symbol_table);
                let line_index = lines.len();
                address_to_line_index.insert(address, line_index);

                lines.push(DisassemblyLine {
                    kind: DisassemblyLineKind::Instruction {
                        instruction: instruction.clone(),
                        target_symbol,
                    },
                    address: Some(address),
                });

                offset += instruction.byte_len();
            } else {
                break;
            }
        }

        Self {
            lines,
            address_to_line_index,
        }
    }

    fn resolve_target_symbol(
        instruction: &DecodedInstruction,
        symbol_table: Option<&SymbolTable>,
    ) -> Option<String> {
        let table = symbol_table?;

        match &instruction.kind {
            InstructionKind::Jci { target_address, .. }
            | InstructionKind::Jmi { target_address, .. }
            | InstructionKind::Jsi { target_address, .. } => table
                .find_nearest_symbol(*target_address)
                .map(|(symbol, offset)| {
                    if offset == 0 {
                        symbol.name().to_string()
                    } else {
                        format!("{}+{offset:02x}", symbol.name())
                    }
                }),
            InstructionKind::Lit2 { value } => table
                .lookup_address(*value)
                .map(|symbol| symbol.name().to_string()),
            _ => None,
        }
    }

    pub fn lines(&self) -> &[DisassemblyLine] {
        &self.lines
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn line_index_for_address(&self, address: u16) -> Option<usize> {
        self.address_to_line_index.get(&address).copied()
    }

    pub fn address_for_line_index(&self, line_index: usize) -> Option<u16> {
        self.lines.get(line_index).and_then(|line| line.address)
    }
}
