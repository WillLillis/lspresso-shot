use lsp_types::DocumentSymbolResponse;

use super::{ApproximateEq, CleanResponse, TestCase, TestExecutionResult, clean_uri};

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

impl ApproximateEq for DocumentSymbolResponse {
    fn approx_eq(a: &Self, b: &Self) -> bool {
        match (a, b) {
            (Self::Flat(sym_info), Self::Nested(doc_syms))
            | (Self::Nested(doc_syms), Self::Flat(sym_info)) => {
                sym_info.is_empty() && doc_syms.is_empty()
            }
            _ => a == b,
        }
    }
}
