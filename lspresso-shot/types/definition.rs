use lsp_types::GotoDefinitionResponse;

use super::{ApproximateEq, CleanResponse, TestCase, TestExecutionResult, clean_uri};

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

// NOTE: The following other requests' response types are simply aliased to this type:
//  - `GotoDeclarationResponse`
//  - `GotoTypeDefinitionResponse`
//  - `GotoImplementationResponse`
impl ApproximateEq for GotoDefinitionResponse {
    fn approx_eq(a: &Self, b: &Self) -> bool {
        match (a, b) {
            (Self::Scalar(a), Self::Scalar(b)) => a == b,
            (Self::Array(a), Self::Array(b)) => a == b,
            (Self::Link(a), Self::Link(b)) => a == b,
            (Self::Array(array), Self::Link(link)) | (Self::Link(link), Self::Array(array)) => {
                link.is_empty() && array.is_empty()
            }
            _ => false,
        }
    }
}
