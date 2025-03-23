use lsp_types::DocumentHighlight;
use thiserror::Error;

use super::{write_fields_comparison, CleanResponse, Empty};

impl Empty for Vec<DocumentHighlight> {}

impl CleanResponse for Vec<DocumentHighlight> {}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct DocumentHighlightMismatchError {
    pub test_id: String,
    pub expected: Vec<DocumentHighlight>,
    pub actual: Vec<DocumentHighlight>,
}

impl std::fmt::Display for DocumentHighlightMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Document Highlight response:",
            self.test_id
        )?;
        write_fields_comparison(f, "Document Highlight", &self.expected, &self.actual, 0)?;
        Ok(())
    }
}
