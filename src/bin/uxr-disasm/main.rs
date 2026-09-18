mod style;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;

use style::{ColorMode, ColorStyle, format_disassembly};
use uxr::shared::file_io::read_file_or_stdin;
use uxr::uxntal::SymbolTable;

fn parse_hex_u16(value: &str) -> Result<u16, String> {
    let cleaned = value.trim_start_matches("0x").trim_start_matches('$');
    u16::from_str_radix(cleaned, 16)
        .map_err(|error| format!("Invalid hexadecimal address '{value}': {error}"))
}

#[derive(Parser, Debug)]
#[command(
    name = "uxr-disasm",
    about = "Disassemble Uxn ROM binaries into objdump-like assembly",
    version
)]
struct CliArguments {
    /// Path to the ROM file (or '-' / omit for stdin)
    #[arg(value_name = "ROM_FILE")]
    rom_path: Option<String>,

    /// Path to symbol (.sym) file
    #[arg(short = 's', long = "sym", value_name = "FILE")]
    symbol_path: Option<PathBuf>,

    /// Do not load symbol file even if present
    #[arg(long = "no-sym")]
    no_sym: bool,

    /// Set starting address in hexadecimal
    #[arg(
        short = 'b',
        long = "base-address",
        value_name = "HEX",
        default_value = "0100",
        value_parser = parse_hex_u16
    )]
    base_address: u16,

    /// Do not suppress consecutive zeroes / BRK instructions
    #[arg(short = 'z', long = "disassemble-zeroes")]
    disassemble_zeroes: bool,

    /// Colorize output
    #[arg(
        long = "color",
        value_name = "WHEN",
        default_value = "auto",
        default_missing_value = "always",
        num_args = 0..=1
    )]
    color_mode: ColorMode,
}

fn auto_discover_symbol_path(rom_path_str: &str) -> Option<PathBuf> {
    let rom_path = Path::new(rom_path_str);

    // Try <rom_path>.sym (e.g., hello.rom.sym)
    let candidate_with_ext = PathBuf::from(format!("{rom_path_str}.sym"));
    if candidate_with_ext.is_file() {
        return Some(candidate_with_ext);
    }

    // Try replacing extension (e.g., hello.sym for hello.rom)
    let candidate_replaced_ext = rom_path.with_extension("sym");
    if candidate_replaced_ext.is_file() {
        return Some(candidate_replaced_ext);
    }

    None
}

fn main() -> Result<()> {
    let cli_arguments = CliArguments::parse();

    let (rom_bytes, file_label) = if let Some(ref path) = cli_arguments.rom_path {
        if path == "-" {
            (read_file_or_stdin()?, "<stdin>".to_string())
        } else {
            let bytes =
                std::fs::read(path).with_context(|| format!("Failed to read ROM file '{path}'"))?;
            (bytes, path.clone())
        }
    } else {
        (read_file_or_stdin()?, "<stdin>".to_string())
    };

    let symbol_table = if cli_arguments.no_sym {
        None
    } else if let Some(explicit_path) = cli_arguments.symbol_path {
        let table = SymbolTable::from_file(&explicit_path)
            .with_context(|| format!("Failed to read symbol file '{}'", explicit_path.display()))?;
        Some(table)
    } else if let Some(ref path) = cli_arguments.rom_path {
        if let Some(discovered_path) = auto_discover_symbol_path(path) {
            SymbolTable::from_file(&discovered_path).ok()
        } else {
            None
        }
    } else {
        None
    };

    let color_style = ColorStyle::new(cli_arguments.color_mode);

    println!("{file_label}:\n");

    let disassembled_text = format_disassembly(
        &rom_bytes,
        symbol_table.as_ref(),
        cli_arguments.base_address,
        cli_arguments.disassemble_zeroes,
        &color_style,
    );
    print!("{disassembled_text}");

    Ok(())
}
