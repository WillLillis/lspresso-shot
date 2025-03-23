use lsp_types::CodeLens;
use thiserror::Error;

use super::{write_fields_comparison, CleanResponse, Empty};

impl Empty for CodeLens {}
impl Empty for Vec<CodeLens> {}

impl CleanResponse for CodeLens {}
impl CleanResponse for Vec<CodeLens> {}

#[derive(Debug, Error, PartialEq)]
pub struct CodeLensMismatchError {
    pub test_id: String,
    pub expected: Vec<CodeLens>,
    pub actual: Vec<CodeLens>,
}

impl std::fmt::Display for CodeLensMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect CodeLens response:", self.test_id)?;
        write_fields_comparison(f, "CodeLens", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq)]
pub struct CodeLensResolveMismatchError {
    pub test_id: String,
    pub expected: CodeLens,
    pub actual: CodeLens,
}

impl std::fmt::Display for CodeLensResolveMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect CodeLens Resolve response:",
            self.test_id
        )?;
        write_fields_comparison(f, "CodeLens", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}
