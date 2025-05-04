use lsp_types::request::GotoImplementationResponse;
use thiserror::Error;

use super::compare::write_fields_comparison;

#[derive(Debug, Error, PartialEq, Eq)]
pub struct ImplementationMismatchError {
    pub test_id: String,
    pub expected: GotoImplementationResponse,
    pub actual: GotoImplementationResponse,
}

impl std::fmt::Display for ImplementationMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Implementation response:",
            self.test_id
        )?;
        write_fields_comparison(
            f,
            "GotoImplementationResponse",
            &self.expected,
            &self.actual,
            0,
        )
    }
}
