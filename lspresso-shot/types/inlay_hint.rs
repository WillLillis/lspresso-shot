use lsp_types::InlayHint;
use thiserror::Error;

use super::compare::Compare as _;

#[derive(Debug, Error, PartialEq)]
pub struct InlayHintMismatchError {
    pub test_id: String,
    pub expected: Vec<InlayHint>,
    pub actual: Vec<InlayHint>,
}

impl std::fmt::Display for InlayHintMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect InlayHint response:",
            self.test_id
        )?;
        <Vec<InlayHint>>::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}
