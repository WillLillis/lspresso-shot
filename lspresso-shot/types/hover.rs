use lsp_types::Hover;
use thiserror::Error;

use super::{CleanResponse, Empty, compare::write_fields_comparison};

impl Empty for Hover {}

impl CleanResponse for Hover {}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct HoverMismatchError {
    pub test_id: String,
    pub expected: Hover,
    pub actual: Hover,
}

impl std::fmt::Display for HoverMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect Hover response:", self.test_id)?;
        write_fields_comparison(f, "Hover", &self.expected, &self.actual, 0)
    }
}
