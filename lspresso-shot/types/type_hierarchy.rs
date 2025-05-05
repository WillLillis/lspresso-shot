use lsp_types::TypeHierarchyItem;
use thiserror::Error;

use super::{
    clean_uri, compare::write_fields_comparison, CleanResponse, Empty, TestCase, TestResult,
};

impl Empty for Vec<TypeHierarchyItem> {}

impl CleanResponse for Vec<TypeHierarchyItem> {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        for item in &mut self {
            item.uri = clean_uri(&item.uri, test_case)?;
        }
        Ok(self)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct PrepareTypeHierarchyMismatchError {
    pub test_id: String,
    pub expected: Vec<TypeHierarchyItem>,
    pub actual: Vec<TypeHierarchyItem>,
}

impl std::fmt::Display for PrepareTypeHierarchyMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Prepare Type Hierarchy response:",
            self.test_id
        )?;
        write_fields_comparison(
            f,
            "Vec<PrepareTypeHierarchy>",
            &self.expected,
            &self.actual,
            0,
        )
    }
}
