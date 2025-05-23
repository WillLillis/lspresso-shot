use lsp_types::TextEdit;

use super::{ApproximateEq, CleanResponse, ResponseMismatchError, StateOrResponse, TestError};

impl CleanResponse for StateOrResponse<Vec<TextEdit>> {}
impl CleanResponse for Vec<TextEdit> {}

impl From<TestError<String>> for TestError<StateOrResponse<Vec<TextEdit>>> {
    fn from(value: TestError<String>) -> Self {
        match value {
            TestError::ResponseMismatch(ResponseMismatchError {
                test_id,
                expected,
                actual,
            }) => {
                let expected = expected.map(StateOrResponse::State);
                let actual = actual.map(StateOrResponse::State);
                Self::ResponseMismatch(ResponseMismatchError {
                    test_id,
                    expected,
                    actual,
                })
            }
            TestError::TestExecution(e) => Self::TestExecution(e),
            TestError::TestSetup(e) => Self::TestSetup(e),
        }
    }
}

impl ApproximateEq for Vec<TextEdit> {}
impl ApproximateEq for StateOrResponse<Vec<TextEdit>> {}
