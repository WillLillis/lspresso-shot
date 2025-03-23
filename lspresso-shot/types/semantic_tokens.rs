use lsp_types::{SemanticTokensFullDeltaResult, SemanticTokensRangeResult, SemanticTokensResult};
use thiserror::Error;

use super::{write_fields_comparison, CleanResponse, Empty};

impl Empty for SemanticTokensResult {}
impl Empty for SemanticTokensFullDeltaResult {}
impl Empty for SemanticTokensRangeResult {}

impl CleanResponse for SemanticTokensResult {}
impl CleanResponse for SemanticTokensFullDeltaResult {}
impl CleanResponse for SemanticTokensRangeResult {}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct SemanticTokensFullMismatchError {
    pub test_id: String,
    pub expected: SemanticTokensResult,
    pub actual: SemanticTokensResult,
}

impl std::fmt::Display for SemanticTokensFullMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Semantic Tokens Full response:",
            self.test_id
        )?;
        write_fields_comparison(f, "Semantic Tokens", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct SemanticTokensFullDeltaMismatchError {
    pub test_id: String,
    pub expected: SemanticTokensFullDeltaResult,
    pub actual: SemanticTokensFullDeltaResult,
}

impl std::fmt::Display for SemanticTokensFullDeltaMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Semantic Tokens Full Delta response:",
            self.test_id
        )?;
        write_fields_comparison(
            f,
            "Semantic Tokens Full Delta",
            &self.expected,
            &self.actual,
            0,
        )?;
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct SemanticTokensRangeMismatchError {
    pub test_id: String,
    pub expected: SemanticTokensRangeResult,
    pub actual: SemanticTokensRangeResult,
}

impl std::fmt::Display for SemanticTokensRangeMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Semantic Tokens Range response:",
            self.test_id
        )?;
        write_fields_comparison(f, "Semantic Tokens", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}
