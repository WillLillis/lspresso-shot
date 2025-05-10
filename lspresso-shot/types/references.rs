use lsp_types::Location;
use thiserror::Error;

use super::{
    CleanResponse, Empty, TestCase, TestResult, clean_uri, compare::write_fields_comparison,
};

impl Empty for Vec<Location> {}

impl CleanResponse for Vec<Location> {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        for loc in &mut self {
            loc.uri = clean_uri(&loc.uri, test_case)?;
        }
        Ok(self)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct ReferencesMismatchError {
    pub test_id: String,
    pub expected: Vec<Location>,
    pub actual: Vec<Location>,
}

impl std::fmt::Display for ReferencesMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect References response:", self.test_id)?;
        write_fields_comparison(f, "Vec<Location>", &self.expected, &self.actual, 0)
    }
}
