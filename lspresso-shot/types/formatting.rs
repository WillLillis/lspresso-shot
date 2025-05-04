use lsp_types::TextEdit;
use serde::Serialize;
use thiserror::Error;

use super::{compare::write_fields_comparison, CleanResponse, Empty};

impl Empty for FormattingResult {}
impl Empty for Vec<TextEdit> {}

impl CleanResponse for FormattingResult {}
impl CleanResponse for Vec<TextEdit> {}

#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
pub enum FormattingResult {
    /// Check if the file's formatted state matches the expected contents
    EndState(String),
    /// Check if the server's response matches the exected edits
    Response(Vec<TextEdit>),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct FormattingMismatchError {
    pub test_id: String,
    pub expected: FormattingResult,
    pub actual: FormattingResult,
}

impl std::fmt::Display for FormattingMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect Formatting response:", self.test_id)?;
        // TODO: This may need some touch up
        write_fields_comparison(f, "FormattingResult", &self.expected, &self.actual, 0)
    }
}
