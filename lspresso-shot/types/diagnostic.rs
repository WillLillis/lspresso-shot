use lsp_types::Diagnostic;
use thiserror::Error;

use super::{clean_uri, write_fields_comparison, CleanResponse, Empty, TestCase, TestResult};

impl Empty for Vec<Diagnostic> {}

impl CleanResponse for Vec<Diagnostic> {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        for diagnostic in &mut self {
            if let Some(info) = diagnostic.related_information.as_mut() {
                for related in info {
                    related.location.uri = clean_uri(&related.location.uri, test_case)?;
                }
            }
        }
        Ok(self)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct DiagnosticMismatchError {
    pub test_id: String,
    pub expected: Vec<Diagnostic>,
    pub actual: Vec<Diagnostic>,
}

impl std::fmt::Display for DiagnosticMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect Diagnostic response:", self.test_id)?;
        write_fields_comparison(f, "Diagnostics", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}
