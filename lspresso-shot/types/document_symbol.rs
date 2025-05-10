use lsp_types::DocumentSymbolResponse;
use thiserror::Error;

use super::{
    CleanResponse, Empty, TestCase, TestResult, clean_uri, compare::write_fields_comparison,
};

impl Empty for DocumentSymbolResponse {}

impl CleanResponse for DocumentSymbolResponse {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        match &mut self {
            Self::Flat(syms) => {
                for sym in syms {
                    sym.location.uri = clean_uri(&sym.location.uri, test_case)?;
                }
            }
            Self::Nested(_) => {}
        }
        Ok(self)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct DocumentSymbolMismatchError {
    pub test_id: String,
    pub expected: DocumentSymbolResponse,
    pub actual: DocumentSymbolResponse,
}

impl std::fmt::Display for DocumentSymbolMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Document Symbol response:",
            self.test_id
        )?;
        write_fields_comparison(f, "DocumentSymbolResponse", &self.expected, &self.actual, 0)
    }
}
