use lsp_types::DocumentLink;

use super::{CleanResponse, TestCase, TestExecutionResult, clean_uri};

impl CleanResponse for DocumentLink {
    fn clean_response(mut self, test_case: &TestCase) -> TestExecutionResult<Self> {
        if let Some(ref mut uri) = self.target {
            *uri = clean_uri(uri, test_case)?;
        }
        Ok(self)
    }
}

impl CleanResponse for Vec<DocumentLink> {
    fn clean_response(mut self, test_case: &TestCase) -> TestExecutionResult<Self> {
        for link in &mut self {
            if let Some(ref mut uri) = link.target {
                *uri = clean_uri(uri, test_case)?;
            }
        }
        Ok(self)
    }
}
