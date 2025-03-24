use lsp_types::{
    CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall, Range, SymbolKind,
    SymbolTag, Uri,
};
use thiserror::Error;

use super::{clean_uri, compare::Compare, CleanResponse, Empty, TestCase, TestResult};

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

#[derive(Debug, Error, PartialEq)]
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
        <Vec<CallHierarchyIncomingCall>>::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

#[derive(Debug, Error, PartialEq)]
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
        <Vec<CallHierarchyOutgoingCall>>::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

#[derive(Debug, Error, PartialEq)]
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
        <Vec<CallHierarchyItem>>::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

impl Compare for CallHierarchyIncomingCall {
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
        writeln!(f, "{padding}{name_str}CallHierarchyIncomingCall {{")?;
        CallHierarchyItem::compare(
            f,
            Some("from"),
            &expected.from,
            &actual.from,
            depth + 1,
            override_color,
        )?;
        <Vec<Range>>::compare(
            f,
            Some("from_ranges"),
            &expected.from_ranges,
            &actual.from_ranges,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for CallHierarchyItem {
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
        writeln!(f, "{padding}{name_str}CallHierarchyItem {{")?;
        <String>::compare(
            f,
            Some("name"),
            &expected.name,
            &actual.name,
            depth + 1,
            override_color,
        )?;
        <SymbolKind>::compare(
            f,
            Some("kind"),
            &expected.kind,
            &actual.kind,
            depth + 1,
            override_color,
        )?;
        <Option<Vec<SymbolTag>>>::compare(
            f,
            Some("tags"),
            &expected.tags,
            &actual.tags,
            depth + 1,
            override_color,
        )?;
        <Option<String>>::compare(
            f,
            Some("detail"),
            &expected.detail,
            &actual.detail,
            depth + 1,
            override_color,
        )?;
        Uri::compare(
            f,
            Some("uri"),
            &expected.uri,
            &actual.uri,
            depth + 1,
            override_color,
        )?;
        Range::compare(
            f,
            Some("range"),
            &expected.range,
            &actual.range,
            depth + 1,
            override_color,
        )?;
        Range::compare(
            f,
            Some("selection_range"),
            &expected.selection_range,
            &actual.selection_range,
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
        writeln!(f, "{padding}}}", padding = "  ".repeat(depth))?;

        Ok(())
    }
}

impl Compare for CallHierarchyOutgoingCall {
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
        writeln!(f, "{padding}{name_str}CallHierarchyOutgoingCall {{")?;
        CallHierarchyItem::compare(
            f,
            Some("to"),
            &expected.to,
            &actual.to,
            depth + 1,
            override_color,
        )?;
        <Vec<Range>>::compare(
            f,
            Some("from_ranges"),
            &expected.from_ranges,
            &actual.from_ranges,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}
