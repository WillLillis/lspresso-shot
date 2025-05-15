use lsp_types::Location;

use super::{CleanResponse, TestCase, TestExecutionResult, clean_uri};

impl CleanResponse for Vec<Location> {
    fn clean_response(mut self, test_case: &TestCase) -> TestExecutionResult<Self> {
        for loc in &mut self {
            loc.uri = clean_uri(&loc.uri, test_case)?;
        }
        Ok(self)
    }
}
