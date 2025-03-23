use lsp_types::request::GotoDeclarationResponse;
use thiserror::Error;

use super::write_fields_comparison;

#[derive(Debug, Error, PartialEq)]
pub struct DeclarationMismatchError {
    pub test_id: String,
    pub expected: GotoDeclarationResponse,
    pub actual: GotoDeclarationResponse,
}

impl std::fmt::Display for DeclarationMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect GotoDeclaration response:",
            self.test_id
        )?;
        write_fields_comparison(f, "GotoDeclaration", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}
