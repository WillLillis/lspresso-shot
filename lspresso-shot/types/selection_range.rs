use lsp_types::SelectionRange;
use thiserror::Error;

use super::{write_fields_comparison, CleanResponse, Empty};

impl Empty for Vec<SelectionRange> {}

impl CleanResponse for Vec<SelectionRange> {}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct SelectionRangeMismatchError {
    pub test_id: String,
    pub expected: Vec<SelectionRange>,
    pub actual: Vec<SelectionRange>,
}

impl std::fmt::Display for SelectionRangeMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Selection Range response:",
            self.test_id
        )?;
        write_fields_comparison(f, "Selection Range", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}
