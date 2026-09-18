use std::fmt;

pub use super::source_map::FileId;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Location {
    pub file_id: FileId,
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

impl Default for Location {
    fn default() -> Self {
        Self {
            file_id: FileId::default(),
            offset: 0,
            line: 1,
            column: 1,
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.line, self.column)
    }
}

impl Location {
    pub fn new(file_id: FileId, offset: usize, line: usize, column: usize) -> Self {
        Self {
            file_id,
            offset,
            line,
            column,
        }
    }

    /// Formats this location as a standard editor link in `file_path:line:column` format.
    pub fn editor_link(&self, file_path: &str) -> String {
        format!("{file_path}:{self}")
    }

    /// Formats a readable source context snippet around this location,
    /// displaying a few lines of context and highlighting the position with a `^` on the next line.
    pub fn format_context(&self, source: &str, context_line_count: usize) -> String {
        let lines: Vec<&str> = source.lines().collect();
        if lines.is_empty() {
            return String::new();
        }

        let target_line_number = self.line.max(1);
        let clamped_target_line = target_line_number.min(lines.len());
        let start_line_number = clamped_target_line
            .saturating_sub(context_line_count)
            .max(1);
        let end_line_number = (clamped_target_line + context_line_count).min(lines.len());

        let gutter_width = end_line_number.to_string().len().max(2);

        let mut output = String::new();

        for current_line_number in start_line_number..=end_line_number {
            let line_content = lines.get(current_line_number - 1).copied().unwrap_or("");
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&format!(
                "{:>gutter_width$} | {line_content}",
                current_line_number
            ));

            if current_line_number == clamped_target_line {
                output.push('\n');
                let mut caret_indentation = String::new();
                for (character_index, character) in line_content.chars().enumerate() {
                    if character_index >= self.column.saturating_sub(1) {
                        break;
                    }
                    if character == '\t' {
                        caret_indentation.push('\t');
                    } else {
                        caret_indentation.push(' ');
                    }
                }

                let character_count = line_content.chars().count();
                let target_column_offset = self.column.saturating_sub(1);
                if target_column_offset > character_count {
                    let extra_spaces = target_column_offset - character_count;
                    caret_indentation.push_str(&" ".repeat(extra_spaces));
                }

                output.push_str(&format!("{:>gutter_width$} | {caret_indentation}^", ""));
            }
        }

        output
    }

    /// Formats a complete error message with a clickable editor link, error message,
    /// a few lines of context from the source asm file, and a caret highlighting the error position.
    pub fn format_error(&self, file_path: &str, source: &str, error_message: &str) -> String {
        let link = self.editor_link(file_path);
        let context = self.format_context(source, 2);
        if context.is_empty() {
            format!("{link}: error: {error_message}")
        } else {
            format!("{link}: error: {error_message}\n{context}")
        }
    }
}
