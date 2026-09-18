use std::fmt::Write;
use std::io::IsTerminal;

use clap::ValueEnum;
use uxr::uxntal::{
    DecodedInstruction, Disassembler, InstructionKind, MnemonicCategory, SymbolTable,
};

/// Color formatting mode for disassembly output.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, ValueEnum)]
pub enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}

impl ColorMode {
    pub fn should_color(&self) -> bool {
        match self {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => {
                std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
            }
        }
    }
}

/// Helper for applying ANSI terminal colors.
#[derive(Debug, Clone, Copy)]
pub struct ColorStyle {
    enabled: bool,
}

impl ColorStyle {
    pub fn new(mode: ColorMode) -> Self {
        Self {
            enabled: mode.should_color(),
        }
    }

    fn colorize(&self, code: &str, text: &str) -> String {
        if self.enabled {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    /// Styles a section or label header (e.g., `00000100 <on-reset>:`).
    pub fn symbol_header(&self, text: &str) -> String {
        self.colorize("1;33", text) // Bold Yellow
    }

    /// Styles an instruction memory address (e.g., `  0100:`).
    pub fn address(&self, text: &str) -> String {
        self.colorize("36", text) // Cyan
    }

    /// Styles raw instruction bytes (e.g., `80 12`).
    pub fn raw_bytes(&self, text: &str) -> String {
        self.colorize("90", text) // Dark Gray / Dim
    }

    /// Styles an opcode mnemonic based on its category.
    pub fn mnemonic(&self, text: &str, category: MnemonicCategory) -> String {
        let code = match category {
            MnemonicCategory::ControlFlow => "1;35",   // Bold Magenta
            MnemonicCategory::Literal => "1;36",       // Bold Cyan
            MnemonicCategory::Arithmetic => "1;32",    // Bold Green
            MnemonicCategory::Comparison => "1;33",    // Bold Yellow
            MnemonicCategory::MemoryOrStack => "1;34", // Bold Blue
            MnemonicCategory::Raw => "1;37",           // Bold White
        };
        self.colorize(code, text)
    }

    /// Styles an immediate target address (e.g., `0108`).
    pub fn target_address(&self, text: &str) -> String {
        self.colorize("32", text) // Green
    }

    /// Styles an immediate literal value (e.g., `#011f`).
    pub fn literal_value(&self, text: &str) -> String {
        self.colorize("33", text) // Yellow
    }

    /// Styles a symbol annotation (e.g., `<hello-world>`).
    pub fn symbol_annotation(&self, text: &str) -> String {
        self.colorize("1;36", text) // Bold Cyan
    }
}

/// Disassembles ROM bytes and formats with the provided ColorStyle.
pub fn format_disassembly(
    rom: &[u8],
    symbol_table: Option<&SymbolTable>,
    base_address: u16,
    disassemble_zeroes: bool,
    style: &ColorStyle,
) -> String {
    let mut output = String::new();
    let mut offset = 0;
    let mut consecutive_zero_count = 0;

    while offset < rom.len() {
        let address = base_address.wrapping_add(offset as u16);

        let has_symbol = symbol_table
            .map(|table| !table.lookup_all_at_address(address).is_empty())
            .unwrap_or(false);

        // Check for zero-suppression (runs of unlabelled 0x00 BRK instructions)
        if !disassemble_zeroes && !has_symbol && rom[offset] == 0x00 {
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
                    let header_text = format!("{:08x} <{}>:", address, symbol.name());
                    output.push_str(&style.symbol_header(&header_text));
                    output.push('\n');
                }
            }
        }

        if let Some(instruction) = Disassembler::decode_instruction(rom, offset, address) {
            let formatted_line = format_instruction_line(&instruction, symbol_table, style);
            output.push_str(&formatted_line);
            output.push('\n');
            offset += instruction.byte_len();
        } else {
            break;
        }
    }

    output
}

fn format_instruction_line(
    instruction: &DecodedInstruction,
    symbol_table: Option<&SymbolTable>,
    style: &ColorStyle,
) -> String {
    let mut line = String::new();

    // 1. Address column: "  0100:"
    let address_plain = format!("  {:04x}:", instruction.address);
    let _ = write!(line, "{}\t", style.address(&address_plain));

    // 2. Raw bytes column: "80 12         " (padded to 10 chars)
    let mut raw_bytes_string = String::new();
    for (byte_index, byte) in instruction.bytes.iter().enumerate() {
        if byte_index > 0 {
            raw_bytes_string.push(' ');
        }
        let _ = write!(raw_bytes_string, "{byte:02x}");
    }
    let raw_bytes_padded = format!("{raw_bytes_string:<10}");
    let _ = write!(line, "{}\t", style.raw_bytes(&raw_bytes_padded));

    // 3. Mnemonic column: "LIT2    " (padded to 8 chars)
    let mnemonic_padded = format!("{:<8}", instruction.mnemonic);
    let _ = write!(
        line,
        "{}",
        style.mnemonic(&mnemonic_padded, instruction.category)
    );

    // 4. Operands & Symbol Annotations
    match &instruction.kind {
        InstructionKind::Jci { target_address, .. }
        | InstructionKind::Jmi { target_address, .. }
        | InstructionKind::Jsi { target_address, .. } => {
            let target_hex = format!("{:04x}", target_address);
            let _ = write!(line, "{}", style.target_address(&target_hex));

            if let Some(annotation) = resolve_symbol_annotation(*target_address, symbol_table) {
                let _ = write!(line, " {}", style.symbol_annotation(&annotation));
            }
        }
        InstructionKind::Lit { value } | InstructionKind::Litr { value } => {
            let value_string = format!("#{value:02x}");
            let _ = write!(line, "{}", style.literal_value(&value_string));

            if let Some(annotation) = resolve_exact_symbol(*value as u16, symbol_table) {
                let _ = write!(line, " {}", style.symbol_annotation(&annotation));
            }
        }
        InstructionKind::Lit2 { value } | InstructionKind::Lit2r { value } => {
            let value_string = format!("#{value:04x}");
            let _ = write!(line, "{}", style.literal_value(&value_string));

            if let Some(annotation) = resolve_symbol_annotation(*value, symbol_table) {
                let _ = write!(line, " {}", style.symbol_annotation(&annotation));
            }
        }
        InstructionKind::Brk | InstructionKind::Regular { .. } | InstructionKind::RawByte(_) => {
            // No additional operands
        }
    }

    line
}

fn resolve_exact_symbol(address: u16, symbol_table: Option<&SymbolTable>) -> Option<String> {
    let table = symbol_table?;
    let symbol = table.lookup_address(address)?;
    Some(format!("<{}>", symbol.name()))
}

fn resolve_symbol_annotation(address: u16, symbol_table: Option<&SymbolTable>) -> Option<String> {
    let table = symbol_table?;

    if let Some(symbol) = table.lookup_address(address) {
        return Some(format!("<{}>", symbol.name()));
    }

    if let Some((symbol, offset)) = table.find_nearest_symbol(address)
        && offset > 0
        && offset <= 0x0100
    {
        return Some(format!("<{}+0x{offset:x}>", symbol.name()));
    }

    None
}
