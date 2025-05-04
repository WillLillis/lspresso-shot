use lsp_types::{CodeActionOrCommand, CodeActionResponse};
use thiserror::Error;

use super::{compare::write_fields_comparison, CleanResponse, Empty, TestResult};

impl Empty for CodeActionResponse {}

impl CleanResponse for CodeActionResponse {
    fn clean_response(mut self, test_case: &super::TestCase) -> TestResult<Self> {
        for action in &mut self {
            match action {
                CodeActionOrCommand::Command(_) => {}
                CodeActionOrCommand::CodeAction(action) => {
                    if let Some(ref mut edit) = action.edit {
                        *edit = edit.clone().clean_response(test_case)?;
                    }
                }
            }
        }
        Ok(self)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct CodeActionMismatchError {
    pub test_id: String,
    pub expected: CodeActionResponse,
    pub actual: CodeActionResponse,
}

impl std::fmt::Display for CodeActionMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect Code Action response:", self.test_id)?;
        write_fields_comparison(f, "CodeActionResponse", &self.expected, &self.actual, 0)
    }
}
