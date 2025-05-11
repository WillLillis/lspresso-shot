use lsp_types::ColorPresentation;
use thiserror::Error;

use super::{CleanResponse, Empty, compare::write_fields_comparison};

impl Empty for Vec<ColorPresentation> {}

impl CleanResponse for Vec<ColorPresentation> {}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct ColorPresentationMismatchError {
    pub test_id: String,
    pub expected: Vec<ColorPresentation>,
    pub actual: Vec<ColorPresentation>,
}

impl std::fmt::Display for ColorPresentationMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Color Presentation response:",
            self.test_id
        )?;
        write_fields_comparison(f, "Vec<ColorPresentation>", &self.expected, &self.actual, 0)
    }
}
