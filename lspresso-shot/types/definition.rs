use lsp_types::GotoDefinitionResponse;
use thiserror::Error;

use super::{clean_uri, compare::Compare as _, CleanResponse, Empty, TestCase, TestResult};

impl Empty for GotoDefinitionResponse {}

impl CleanResponse for GotoDefinitionResponse {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
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

#[derive(Debug, Error, PartialEq)]
pub struct DefinitionMismatchError {
    pub test_id: String,
    pub expected: GotoDefinitionResponse,
    pub actual: GotoDefinitionResponse,
}

impl std::fmt::Display for DefinitionMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect GotoDefinition response:",
            self.test_id
        )?;
        GotoDefinitionResponse::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}
