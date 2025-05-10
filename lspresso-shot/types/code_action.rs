use lsp_types::{CodeAction, CodeActionOrCommand, CodeActionResponse};
use thiserror::Error;

use super::{CleanResponse, Empty, TestResult, compare::write_fields_comparison};

impl Empty for CodeActionResponse {}
impl Empty for CodeAction {}

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

impl CleanResponse for CodeAction {
    fn clean_response(mut self, test_case: &super::TestCase) -> TestResult<Self> {
        if let Some(ref mut edit) = self.edit {
            *edit = edit.clone().clean_response(test_case)?;
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

#[derive(Debug, Error, PartialEq, Eq)]
pub struct CodeActionResolveMismatchError {
    pub test_id: String,
    pub expected: CodeAction,
    pub actual: CodeAction,
}

impl std::fmt::Display for CodeActionResolveMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Code Action Resolve response:",
            self.test_id
        )?;
        write_fields_comparison(f, "CodeAction", &self.expected, &self.actual, 0)
    }
}
