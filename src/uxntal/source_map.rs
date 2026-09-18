use std::fmt;

use super::location::Location;

/// Unique identifier for a source file registered in a [`SourceMap`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct FileId(pub usize);

impl FileId {
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    pub const fn index(self) -> usize {
        self.0
    }
}

impl fmt::Display for FileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Represents a source file loaded into the system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub id: FileId,
    pub path: String,
    pub source: String,
}

impl SourceFile {
    pub fn new(id: FileId, path: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            id,
            path: path.into(),
            source: source.into(),
        }
    }

    pub fn id(&self) -> FileId {
        self.id
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

/// Registry of all source files compiled during an assembly run.
///
/// Tracks file paths and their contents, mapping each to a unique [`FileId`].
/// This allows tokens and AST statements to carry lightweight `Copy` locations
/// without duplicating path strings, while enabling rich diagnostic formatting
/// across multiple included files.
#[derive(Debug, Default, Clone)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    /// Adds a source file to the map. If a file with the same path already exists,
    /// returns its existing [`FileId`].
    pub fn add_file(&mut self, path: impl Into<String>, source: impl Into<String>) -> FileId {
        let path = path.into();
        let source = source.into();

        if let Some(existing) = self.files.iter().find(|file| file.path == path) {
            return existing.id;
        }

        let id = FileId(self.files.len());
        self.files.push(SourceFile::new(id, path, source));
        id
    }

    pub fn get_file(&self, id: FileId) -> Option<&SourceFile> {
        self.files.get(id.0)
    }

    pub fn get_path(&self, id: FileId) -> Option<&str> {
        self.get_file(id).map(|file| file.path())
    }

    pub fn get_source(&self, id: FileId) -> Option<&str> {
        self.get_file(id).map(|file| file.source())
    }

    pub fn files(&self) -> &[SourceFile] {
        &self.files
    }

    /// Formats an editor link for the given location using the file path registered in this source map.
    pub fn editor_link(&self, location: Location) -> String {
        let path = self.get_path(location.file_id).unwrap_or("<unknown>");
        location.editor_link(path)
    }

    /// Formats a source context snippet for the given location using the source registered in this source map.
    pub fn format_context(&self, location: Location, context_line_count: usize) -> String {
        match self.get_source(location.file_id) {
            Some(source) => location.format_context(source, context_line_count),
            None => String::new(),
        }
    }

    /// Formats a complete error message with editor link, context snippet, and caret.
    pub fn format_error(&self, location: Location, error_message: &str) -> String {
        let path = self.get_path(location.file_id).unwrap_or("<unknown>");
        match self.get_source(location.file_id) {
            Some(source) => location.format_error(path, source, error_message),
            None => {
                let link = location.editor_link(path);
                format!("{link}: error: {error_message}")
            }
        }
    }
}
