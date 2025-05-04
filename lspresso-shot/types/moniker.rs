use lsp_types::Moniker;
use thiserror::Error;

use super::{compare::write_fields_comparison, CleanResponse, Empty};

impl Empty for Vec<Moniker> {}
impl CleanResponse for Vec<Moniker> {}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct MonikerMismatchError {
    pub test_id: String,
    pub expected: Vec<Moniker>,
    pub actual: Vec<Moniker>,
}

impl std::fmt::Display for MonikerMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect Moniker response:", self.test_id)?;
        write_fields_comparison(f, "Vec<Moniker>", &self.expected, &self.actual, 0)
    }
}
