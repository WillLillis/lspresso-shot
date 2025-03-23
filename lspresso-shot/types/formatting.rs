use lsp_types::TextEdit;
use thiserror::Error;

use super::{write_fields_comparison, CleanResponse, Empty};

impl Empty for FormattingResult {}
impl Empty for Vec<TextEdit> {}

impl CleanResponse for FormattingResult {}
impl CleanResponse for Vec<TextEdit> {}

#[derive(Debug, PartialEq, Eq, Clone)]
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
        match (&self.expected, &self.actual) {
            (
                FormattingResult::Response(expected_edits),
                FormattingResult::Response(actual_edits),
            ) => {
                write_fields_comparison(f, "TextEdit", expected_edits, actual_edits, 0)?;
            }
            (
                FormattingResult::EndState(expected_end_state),
                FormattingResult::EndState(actual_end_state),
            ) => {
                write_fields_comparison(f, "EndState", expected_end_state, actual_end_state, 0)?;
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}
