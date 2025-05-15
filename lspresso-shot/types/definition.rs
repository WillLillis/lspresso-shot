use lsp_types::GotoDefinitionResponse;

use super::{CleanResponse, TestCase, TestExecutionResult, clean_uri};

impl CleanResponse for GotoDefinitionResponse {
    fn clean_response(mut self, test_case: &TestCase) -> TestExecutionResult<Self> {
        match &mut self {
            Self::Scalar(location) => {
                location.uri = clean_uri(&location.uri, test_case)?;
            }
            Self::Array(locs) => {
                for loc in locs {
                    loc.uri = clean_uri(&loc.uri, test_case)?;
                }
            }
            Self::Link(links) => {
                for link in links {
                    link.target_uri = clean_uri(&link.target_uri, test_case)?;
                }
            }
        }
        Ok(self)
    }
}
