use lsp_types::{CompletionItem, CompletionResponse};
use thiserror::Error;

use super::{compare::write_fields_comparison, CleanResponse, Empty};

impl Empty for CompletionResponse {}
impl Empty for CompletionItem {}

impl CleanResponse for CompletionResponse {}
impl CleanResponse for CompletionItem {}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct CompletionResolveMismatchError {
    pub test_id: String,
    pub expected: CompletionItem,
    pub actual: CompletionItem,
}

impl std::fmt::Display for CompletionResolveMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect CompletionResolve response:",
            self.test_id
        )?;
        write_fields_comparison(f, "CompletionResolve", &self.expected, &self.actual, 0)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct CompletionMismatchError {
    pub test_id: String,
    pub expected: CompletionResponse,
    pub actual: CompletionResponse,
}

impl std::fmt::Display for CompletionMismatchError {
    #[allow(clippy::too_many_lines)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect Completion response:", self.test_id)?;
        write_fields_comparison(f, "Completion", &self.expected, &self.actual, 0)
    }
}
