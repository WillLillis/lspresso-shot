use lsp_types::TextEdit;
use serde::Serialize;

use super::{CleanResponse, ResponseMismatchError, TestError, TestResult};

impl CleanResponse for FormattingResult {}
impl CleanResponse for Vec<TextEdit> {}

#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
pub enum FormattingResult {
    /// Check if the file's formatted state matches the expected contents
    EndState(String),
    /// Check if the server's response matches the exected edits
    Response(Vec<TextEdit>),
}

/// Converts a `TestResult<(), String>` or `TestResult<(), Vec<TextEdit>>` to `TestResult<(), FormattingResult>`.
/// This is necessary to satisfy the generic constraints introduced by `test_formatting` calling
/// `test_formatting_resp` and `test_formatting_state`.
///
/// Note that we can't implement this logic directly via the `From` trait because `TestResult` is just an alias
/// for `Result`, and thus a foreign type.
pub(crate) fn to_parent_err_type<T>(result: TestResult<(), T>) -> TestResult<(), FormattingResult>
where
    TestError<FormattingResult>: From<TestError<T>>,
{
    match result {
        Ok(()) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

impl From<TestError<Vec<TextEdit>>> for TestError<FormattingResult> {
    fn from(value: TestError<Vec<TextEdit>>) -> Self {
        match value {
            TestError::ResponseMismatch(ResponseMismatchError {
                test_id,
                expected,
                actual,
            }) => {
                let expected = expected.map(FormattingResult::Response);
                let actual = actual.map(FormattingResult::Response);
                Self::ResponseMismatch(ResponseMismatchError {
                    test_id,
                    expected,
                    actual,
                })
            }
            TestError::TestSetup(e) => Self::TestSetup(e),
            TestError::TestExecution(e) => Self::TestExecution(e),
        }
    }
}

impl From<TestError<String>> for TestError<FormattingResult> {
    fn from(value: TestError<String>) -> Self {
        match value {
            TestError::ResponseMismatch(ResponseMismatchError {
                test_id,
                expected,
                actual,
            }) => {
                let expected = expected.map(FormattingResult::EndState);
                let actual = actual.map(FormattingResult::EndState);
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
