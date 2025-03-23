use lsp_types::FoldingRange;
use thiserror::Error;

use super::{write_fields_comparison, CleanResponse, Empty};

impl Empty for Vec<FoldingRange> {}

impl CleanResponse for Vec<FoldingRange> {}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct FoldingRangeMismatchError {
    pub test_id: String,
    pub expected: Vec<FoldingRange>,
    pub actual: Vec<FoldingRange>,
}

impl std::fmt::Display for FoldingRangeMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Folding Range response:", self.test_id)?;
        write_fields_comparison(f, "FoldingRange", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}
