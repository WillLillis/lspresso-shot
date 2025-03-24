use lsp_types::{DocumentLink, Range, Uri};
use thiserror::Error;

use super::{clean_uri, compare::Compare, CleanResponse, Empty, TestCase, TestResult};

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
        <Vec<DocumentLink>>::compare(f, None, &self.expected, &self.actual, 0, None)
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
        DocumentLink::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

impl Compare for DocumentLink {
    type Nested1 = ();
    type Nested2 = ();
    fn compare(
        f: &mut std::fmt::Formatter<'_>,
        name: Option<&str>,
        expected: &Self,
        actual: &Self,
        depth: usize,
        override_color: Option<anstyle::Color>,
    ) -> std::fmt::Result {
        let padding = "  ".repeat(depth);
        let name_str = name.map_or_else(String::new, |name| format!("{name}: "));
        writeln!(f, "{padding}{name_str}DocumentLink {{")?;
        Range::compare(
            f,
            Some("range"),
            &expected.range,
            &actual.range,
            depth + 1,
            override_color,
        )?;
        <Option<Uri>>::compare(
            f,
            Some("target"),
            &expected.target,
            &actual.target,
            depth + 1,
            override_color,
        )?;
        <Option<String>>::compare(
            f,
            Some("tooltip"),
            &expected.tooltip,
            &actual.tooltip,
            depth + 1,
            override_color,
        )?;
        <Option<serde_json::Value>>::compare(
            f,
            Some("data"),
            &expected.data,
            &actual.data,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}
