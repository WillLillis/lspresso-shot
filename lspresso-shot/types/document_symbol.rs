use lsp_types::DocumentSymbolResponse;

use super::{CleanResponse, TestCase, TestExecutionResult, clean_uri};

impl CleanResponse for DocumentSymbolResponse {
    fn clean_response(mut self, test_case: &TestCase) -> TestExecutionResult<Self> {
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
