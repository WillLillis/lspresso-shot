use lsp_types::DocumentLink;
use thiserror::Error;

use super::{
    CleanResponse, Empty, TestCase, TestResult, clean_uri, compare::write_fields_comparison,
};

impl Empty for DocumentLink {}
impl Empty for Vec<DocumentLink> {}

impl CleanResponse for DocumentLink {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        if let Some(ref mut uri) = self.target {
            *uri = clean_uri(uri, test_case)?;
        }
        Ok(self)
    }
}

impl CleanResponse for Vec<DocumentLink> {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        for link in &mut self {
            if let Some(ref mut uri) = link.target {
                *uri = clean_uri(uri, test_case)?;
            }
        }
        Ok(self)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct DocumentLinkMismatchError {
    pub test_id: String,
    pub expected: Vec<DocumentLink>,
    pub actual: Vec<DocumentLink>,
}

impl std::fmt::Display for DocumentLinkMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Document Link response:",
            self.test_id
        )?;
        write_fields_comparison(f, "Vec<DocumentLink>", &self.expected, &self.actual, 0)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct DocumentLinkResolveMismatchError {
    pub test_id: String,
    pub expected: DocumentLink,
    pub actual: DocumentLink,
}

impl std::fmt::Display for DocumentLinkResolveMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Document Link Resolve response:",
            self.test_id
        )?;
        write_fields_comparison(f, "DocumentLink", &self.expected, &self.actual, 0)
    }
}
