use lsp_types::SignatureHelp;
use thiserror::Error;

use super::{compare::write_fields_comparison, CleanResponse, Empty};

#[derive(Debug, Error, PartialEq, Eq)]
pub struct SignatureHelpMismatchError {
    pub test_id: String,
    pub expected: SignatureHelp,
    pub actual: SignatureHelp,
}

impl std::fmt::Display for SignatureHelpMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Signature Help response:",
            self.test_id
        )?;
        write_fields_comparison(f, "SignatureHelp", &self.expected, &self.actual, 0)
    }
}

impl Empty for SignatureHelp {}
impl CleanResponse for SignatureHelp {}
