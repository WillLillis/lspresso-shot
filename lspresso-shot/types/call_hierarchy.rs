use lsp_types::{CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall};
use thiserror::Error;

use super::{
    CleanResponse, Empty, TestCase, TestResult, clean_uri, compare::write_fields_comparison,
};

impl Empty for Vec<CallHierarchyItem> {}
impl Empty for Vec<CallHierarchyIncomingCall> {}
impl Empty for Vec<CallHierarchyOutgoingCall> {}

impl CleanResponse for Vec<CallHierarchyItem> {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        for item in &mut self {
            item.uri = clean_uri(&item.uri, test_case)?;
        }
        Ok(self)
    }
}
impl CleanResponse for Vec<CallHierarchyIncomingCall> {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        for call in &mut self {
            call.from.uri = clean_uri(&call.from.uri, test_case)?;
        }
        Ok(self)
    }
}
impl CleanResponse for Vec<CallHierarchyOutgoingCall> {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        for call in &mut self {
            call.to.uri = clean_uri(&call.to.uri, test_case)?;
        }
        Ok(self)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct IncomingCallsMismatchError {
    pub test_id: String,
    pub expected: Vec<CallHierarchyIncomingCall>,
    pub actual: Vec<CallHierarchyIncomingCall>,
}

impl std::fmt::Display for IncomingCallsMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect IncomingCalls response:",
            self.test_id
        )?;
        write_fields_comparison(f, "IncomingCalls", &self.expected, &self.actual, 0)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct OutgoingCallsMismatchError {
    pub test_id: String,
    pub expected: Vec<CallHierarchyOutgoingCall>,
    pub actual: Vec<CallHierarchyOutgoingCall>,
}

impl std::fmt::Display for OutgoingCallsMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect OutgoingCalls response:",
            self.test_id
        )?;
        write_fields_comparison(f, "OutgoingCalls", &self.expected, &self.actual, 0)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct PrepareCallHierachyMismatchError {
    pub test_id: String,
    pub expected: Vec<CallHierarchyItem>,
    pub actual: Vec<CallHierarchyItem>,
}

impl std::fmt::Display for PrepareCallHierachyMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Prepare Call Hierarchy response:",
            self.test_id
        )?;
        write_fields_comparison(f, "PrepareCallHierarchy", &self.expected, &self.actual, 0)
    }
}
