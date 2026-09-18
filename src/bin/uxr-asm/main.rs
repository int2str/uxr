use anyhow::{Context, Result};

use uxr::shared::file_io::{FsFileLoader, read_file_or_stdin};
use uxr::uxntal::{
    AssembledProgram, Assembler, Parser, Preprocessor, SourceMap, Tokenizer, UxntalError,
};

fn main() -> Result<()> {
    let arguments: Vec<String> = std::env::args().collect();
    let file_path = arguments
        .get(1)
        .map(|argument| argument.as_str())
        .filter(|argument| *argument != "-")
        .unwrap_or("<stdin>");

    let bytes = read_file_or_stdin()?;
    let source = std::str::from_utf8(&bytes).context("Source is not valid UTF-8")?;

    let mut source_map = SourceMap::new();

    let program = match assemble(&mut source_map, file_path, source) {
        Ok(program) => program,
        Err(error) => {
            let formatted_error = error.format_error(&source_map);
            eprintln!("{formatted_error}");
            std::process::exit(1);
        }
    };

    if let Some(output_path) = arguments.get(2) {
        if output_path == "-" {
            use std::io::Write;
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(&program.rom)?;
        } else {
            std::fs::write(output_path, &program.rom)
                .with_context(|| format!("Failed to write ROM to '{output_path}'"))?;

            let symbol_path = format!("{output_path}.sym");
            std::fs::write(&symbol_path, program.symbols.to_bytes())
                .with_context(|| format!("Failed to write symbols to '{symbol_path}'"))?;
        }
    } else {
        hex_dump(&program.rom);
    }

    Ok(())
}

fn assemble(
    source_map: &mut SourceMap,
    file_path: &str,
    source: &str,
) -> Result<AssembledProgram, UxntalError> {
    let tokens = Tokenizer::tokenize(source_map, file_path, source);
    let tokens = Preprocessor::preprocess(source_map, &tokens, FsFileLoader)?;
    let statements = Parser::parse(&tokens)?;
    let program = Assembler::assemble_with_symbols(&statements)?;

    Ok(program)
}

fn hex_dump(rom: &[u8]) {
    for (chunk_index, chunk) in rom.chunks(16).enumerate() {
        let address = chunk_index * 16;
        print!("{address:04x}:");

        for (byte_index, byte) in chunk.iter().enumerate() {
            if byte_index == 8 {
                print!(" ");
            }
            print!(" {byte:02x}");
        }

        if chunk.len() < 16 {
            for byte_index in chunk.len()..16 {
                if byte_index == 8 {
                    print!(" ");
                }
                print!("   ");
            }
        }

        print!("  |");
        for byte in chunk {
            if (0x20..=0x7e).contains(byte) {
                print!("{}", *byte as char)
            } else {
                print!(".");
            }
        }
        println!("|");
    }
}
