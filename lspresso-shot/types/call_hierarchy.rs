use lsp_types::{CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall};

use super::{CleanResponse, TestCase, TestExecutionResult, clean_uri};

impl CleanResponse for Vec<CallHierarchyItem> {
    fn clean_response(mut self, test_case: &TestCase) -> TestExecutionResult<Self> {
        for item in &mut self {
            item.uri = clean_uri(&item.uri, test_case)?;
        }
        Ok(self)
    }
}
impl CleanResponse for Vec<CallHierarchyIncomingCall> {
    fn clean_response(mut self, test_case: &TestCase) -> TestExecutionResult<Self> {
        for call in &mut self {
            call.from.uri = clean_uri(&call.from.uri, test_case)?;
        }
        Ok(self)
    }
}
impl CleanResponse for Vec<CallHierarchyOutgoingCall> {
    fn clean_response(mut self, test_case: &TestCase) -> TestExecutionResult<Self> {
        for call in &mut self {
            call.to.uri = clean_uri(&call.to.uri, test_case)?;
        }
        Ok(self)
    }
}
