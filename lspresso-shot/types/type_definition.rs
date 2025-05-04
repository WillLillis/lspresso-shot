use lsp_types::request::GotoTypeDefinitionResponse;
use thiserror::Error;

use super::compare::write_fields_comparison;

#[derive(Debug, Error, PartialEq, Eq)]
pub struct TypeDefinitionMismatchError {
    pub test_id: String,
    pub expected: GotoTypeDefinitionResponse,
    pub actual: GotoTypeDefinitionResponse,
}

impl std::fmt::Display for TypeDefinitionMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect GotoTypeDefinition response:",
            self.test_id
        )?;
        write_fields_comparison(
            f,
            "GotoTypeDefinitionResponse",
            &self.expected,
            &self.actual,
            0,
        )
    }
}
