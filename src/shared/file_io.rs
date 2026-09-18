//! File and standard I/O helper functions

use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

use thiserror::Error;

/// Trait for loading external files (e.g. for `~include` directives).
///
/// Decouples higher-level logic (like the preprocessor) from the underlying filesystem and OS,
/// making file access OS-independent, testable, and mockable.
pub trait FileLoader {
    fn read(&mut self, path: &str) -> io::Result<String>;
}

/// Blanket implementation allowing any closure `FnMut(&str) -> io::Result<String>`
/// to be passed directly as a `FileLoader`.
impl<F> FileLoader for F
where
    F: FnMut(&str) -> io::Result<String>,
{
    fn read(&mut self, path: &str) -> io::Result<String> {
        self(path)
    }
}

/// A no-op file loader that fails any read attempt.
///
/// Useful when file includes are not supported or when running in environments
/// where filesystem access is disabled.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct NullFileLoader;

impl FileLoader for NullFileLoader {
    fn read(&mut self, path: &str) -> io::Result<String> {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("file loading not configured; cannot read '{path}'"),
        ))
    }
}

/// Standard file loader reading from the local filesystem using `std::fs`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FsFileLoader;

impl FileLoader for FsFileLoader {
    fn read(&mut self, path: &str) -> io::Result<String> {
        fs::read_to_string(path)
    }
}

#[derive(Error, Debug)]
pub enum CliError {
    #[error("failed to read binary data from {source}")]
    ReadInput {
        source: String,
        #[source]
        source_error: io::Error,
    },
}

/// Reads binary data from a file if passed as the first command-line argument,
/// otherwise reads from standard input until EOF.
pub fn read_file_or_stdin() -> Result<Vec<u8>, CliError> {
    let arguments: Vec<String> = env::args().collect();
    let file_path = arguments.get(1).map(|argument| argument.as_str());

    let source = file_path.unwrap_or("stdin");
    read_binary_input(file_path).map_err(|error| CliError::ReadInput {
        source: source.to_string(),
        source_error: error,
    })
}

/// Reads binary data from a file if `path` is provided (and not `"-"`),
/// otherwise reads from standard input until EOF.
fn read_binary_input<P: AsRef<Path>>(path: Option<P>) -> io::Result<Vec<u8>> {
    match path {
        Some(ref target_path) if target_path.as_ref() != Path::new("-") => fs::read(target_path),
        _ => read_from_reader(&mut io::stdin().lock()),
    }
}

/// Reads all bytes from a reader into a byte vector.
fn read_from_reader<R: Read>(reader: &mut R) -> io::Result<Vec<u8>> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    Ok(buffer)
}
