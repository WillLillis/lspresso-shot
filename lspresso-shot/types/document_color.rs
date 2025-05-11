use lsp_types::ColorInformation;
use thiserror::Error;

use super::{CleanResponse, Empty, compare::write_fields_comparison};

impl Empty for Vec<ColorInformation> {}

impl CleanResponse for Vec<ColorInformation> {}

#[derive(Debug, Error, PartialEq)]
pub struct DocumentColorMismatchError {
    pub test_id: String,
    pub expected: Vec<ColorInformation>,
    pub actual: Vec<ColorInformation>,
}

impl std::fmt::Display for DocumentColorMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Document Color response:",
            self.test_id
        )?;
        write_fields_comparison(f, "Vec<ColorInformation>", &self.expected, &self.actual, 0)
    }
}
