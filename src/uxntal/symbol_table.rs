use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum SymbolTableError {
    #[error("Unexpected end of file while reading symbol at byte offset {offset}")]
    UnexpectedEndOfFile { offset: usize },

    #[error("Invalid UTF-8 in symbol name at byte offset {offset}")]
    InvalidUtf8 { offset: usize },

    #[error("I/O error: {0}")]
    Io(String),
}

impl From<std::io::Error> for SymbolTableError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

/// A named memory address symbol in Uxntal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    name: String,
    address: u16,
}

impl Symbol {
    pub fn new(name: impl Into<String>, address: u16) -> Self {
        Self {
            name: name.into(),
            address,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn address(&self) -> u16 {
        self.address
    }
}

/// A collection of symbols mapping names to 16-bit memory addresses.
///
/// Preserves the original definition order of symbols while providing fast lookups.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SymbolTable {
    symbols: Vec<Symbol>,
    address_to_symbol_indices: HashMap<u16, Vec<usize>>,
    name_to_symbol_index: HashMap<String, usize>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a symbol into the table.
    ///
    /// If a symbol with the same name already exists, the entry is updated in the name index,
    /// and added to the address index.
    pub fn insert(&mut self, name: impl Into<String>, address: u16) {
        let name_string = name.into();
        let index = self.symbols.len();

        self.address_to_symbol_indices
            .entry(address)
            .or_default()
            .push(index);

        self.name_to_symbol_index.insert(name_string.clone(), index);

        self.symbols.push(Symbol {
            name: name_string,
            address,
        });
    }

    /// Returns a slice of all symbols in definition order.
    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    /// Returns the number of symbols in the table.
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// Returns whether the table is empty.
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Looks up the primary (first defined) symbol at the exact address.
    pub fn lookup_address(&self, address: u16) -> Option<&Symbol> {
        self.address_to_symbol_indices
            .get(&address)
            .and_then(|indices| indices.first())
            .map(|&index| &self.symbols[index])
    }

    /// Looks up all symbols defined at the exact address.
    pub fn lookup_all_at_address(&self, address: u16) -> Vec<&Symbol> {
        self.address_to_symbol_indices
            .get(&address)
            .map(|indices| indices.iter().map(|&index| &self.symbols[index]).collect())
            .unwrap_or_default()
    }

    /// Looks up a symbol's address by its full name.
    pub fn lookup_name(&self, name: &str) -> Option<u16> {
        self.name_to_symbol_index
            .get(name)
            .map(|&index| self.symbols[index].address())
    }

    /// Finds the nearest preceding or exact symbol for a given address.
    ///
    /// Returns a reference to the symbol along with the byte offset from that symbol.
    pub fn find_nearest_symbol(&self, address: u16) -> Option<(&Symbol, u16)> {
        let mut best_match: Option<(&Symbol, u16)> = None;

        for symbol in &self.symbols {
            if symbol.address <= address {
                let offset = address - symbol.address;
                match best_match {
                    None => best_match = Some((symbol, offset)),
                    Some((_, best_offset)) if offset < best_offset => {
                        best_match = Some((symbol, offset));
                    }
                    _ => {}
                }
            }
        }

        best_match
    }

    /// Serializes the symbol table to bytes in reference `.sym` format.
    ///
    /// Each entry contains:
    /// - 2 bytes: address (big-endian)
    /// - variable bytes: symbol name
    /// - 1 byte: 0x00 (null terminator)
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for symbol in &self.symbols {
            bytes.push((symbol.address >> 8) as u8);
            bytes.push((symbol.address & 0xff) as u8);
            bytes.extend_from_slice(symbol.name.as_bytes());
            bytes.push(0x00);
        }
        bytes
    }

    /// Deserializes a symbol table from bytes in reference `.sym` format.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SymbolTableError> {
        let mut table = Self::new();
        let mut position = 0;

        while position < bytes.len() {
            if position + 2 > bytes.len() {
                return Err(SymbolTableError::UnexpectedEndOfFile { offset: position });
            }

            let high_byte = bytes[position];
            let low_byte = bytes[position + 1];
            let address = ((high_byte as u16) << 8) | (low_byte as u16);
            position += 2;

            let name_start = position;
            while position < bytes.len() && bytes[position] != 0x00 {
                position += 1;
            }

            if position >= bytes.len() {
                return Err(SymbolTableError::UnexpectedEndOfFile { offset: position });
            }

            let name_bytes = &bytes[name_start..position];
            let name = std::str::from_utf8(name_bytes)
                .map_err(|_| SymbolTableError::InvalidUtf8 { offset: name_start })?
                .to_string();

            // Skip null terminator
            position += 1;

            table.insert(name, address);
        }

        Ok(table)
    }

    /// Writes the symbol table to a file at the specified path.
    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SymbolTableError> {
        let mut file = File::create(path)?;
        file.write_all(&self.to_bytes())?;
        Ok(())
    }

    /// Reads a symbol table from a file at the specified path.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, SymbolTableError> {
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Self::from_bytes(&bytes)
    }
}
