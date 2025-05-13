use lsp_types::LinkedEditingRanges;
use thiserror::Error;

use super::{CleanResponse, Empty, compare::write_fields_comparison};

impl Empty for LinkedEditingRanges {}
impl CleanResponse for LinkedEditingRanges {}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct LinkedEditingRangeMismatchError {
    pub test_id: String,
    pub expected: LinkedEditingRanges,
    pub actual: LinkedEditingRanges,
}

impl std::fmt::Display for LinkedEditingRangeMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Linked Editing Range response:",
            self.test_id
        )?;
        write_fields_comparison(f, "LinkedEditingRanges", &self.expected, &self.actual, 0)
    }
}
