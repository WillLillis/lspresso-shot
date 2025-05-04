use lsp_types::InlayHint;
use thiserror::Error;

use super::{compare::write_fields_comparison, CleanResponse, Empty};

impl Empty for Vec<InlayHint> {}
impl CleanResponse for Vec<InlayHint> {}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct InlayHintMismatchError {
    pub test_id: String,
    pub expected: Vec<InlayHint>,
    pub actual: Vec<InlayHint>,
}

impl std::fmt::Display for InlayHintMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect Inlay Hint response:", self.test_id)?;
        write_fields_comparison(f, "Vec<InlayHint>", &self.expected, &self.actual, 0)
    }
}
