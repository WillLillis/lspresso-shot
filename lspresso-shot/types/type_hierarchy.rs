use lsp_types::TypeHierarchyItem;

use super::{ApproximateEq, CleanResponse, TestCase, TestExecutionResult, clean_uri};

impl CleanResponse for Vec<TypeHierarchyItem> {
    fn clean_response(mut self, test_case: &TestCase) -> TestExecutionResult<Self> {
        for item in &mut self {
            item.uri = clean_uri(&item.uri, test_case)?;
        }
        Ok(self)
    }
}

impl ApproximateEq for Vec<TypeHierarchyItem> {}
