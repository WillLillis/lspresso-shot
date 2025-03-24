use lsp_types::request::GotoImplementationResponse;
use thiserror::Error;

use super::compare::Compare as _;

#[derive(Debug, Error, PartialEq)]
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
        GotoImplementationResponse::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}
