use std::collections::HashMap;

use lsp_types::{
    CodeDescription, Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, DiagnosticTag,
    DocumentDiagnosticReport, DocumentDiagnosticReportKind, FullDocumentDiagnosticReport, Location,
    NumberOrString, Range, RelatedFullDocumentDiagnosticReport,
    RelatedUnchangedDocumentDiagnosticReport, UnchangedDocumentDiagnosticReport, Uri,
};
use thiserror::Error;

use super::{
    clean_uri,
    compare::{cmp_fallback, Compare},
    CleanResponse, Empty, TestCase, TestResult,
};

impl Empty for Vec<Diagnostic> {}
impl Empty for DocumentDiagnosticReport {}

impl CleanResponse for Vec<Diagnostic> {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        for diagnostic in &mut self {
            if let Some(info) = diagnostic.related_information.as_mut() {
                for related in info {
                    related.location.uri = clean_uri(&related.location.uri, test_case)?;
                }
            }
        }
        Ok(self)
    }
}

impl CleanResponse for DocumentDiagnosticReportKind {}

impl CleanResponse for DocumentDiagnosticReport {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        match &mut self {
            Self::Full(report) => {
                if let Some(ref mut related_documents) = report.related_documents {
                    let mut cleaned_map = HashMap::new();
                    for (uri, kind) in related_documents.drain() {
                        let cleaned_uri = clean_uri(&uri, test_case)?;
                        cleaned_map.insert(cleaned_uri, kind);
                    }
                    *related_documents = cleaned_map;
                }
            }
            Self::Unchanged(report) => {
                if let Some(ref mut related_documents) = report.related_documents {
                    let mut cleaned_map = HashMap::new();
                    for (uri, kind) in related_documents.drain() {
                        let cleaned_uri = clean_uri(&uri, test_case)?;
                        cleaned_map.insert(cleaned_uri, kind);
                    }
                    *related_documents = cleaned_map;
                }
            }
        }

        Ok(self)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct PublishDiagnosticsMismatchError {
    pub test_id: String,
    pub expected: Vec<Diagnostic>,
    pub actual: Vec<Diagnostic>,
}

impl std::fmt::Display for PublishDiagnosticsMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Publish Diagnostics response:",
            self.test_id
        )?;
        <Vec<Diagnostic>>::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct DiagnosticMismatchError {
    pub test_id: String,
    pub expected: DocumentDiagnosticReport,
    pub actual: DocumentDiagnosticReport,
}

impl std::fmt::Display for DiagnosticMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect Diagnostic response:", self.test_id)?;
        DocumentDiagnosticReport::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

impl Compare for DocumentDiagnosticReport {
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
        match (expected, actual) {
            (Self::Full(expected_full), Self::Full(actual_full)) => {
                RelatedFullDocumentDiagnosticReport::compare(
                    f,
                    name,
                    expected_full,
                    actual_full,
                    depth,
                    override_color,
                )?;
            }
            (Self::Unchanged(expected_unchanged), Self::Unchanged(actual_unchanged)) => {
                RelatedUnchangedDocumentDiagnosticReport::compare(
                    f,
                    name,
                    expected_unchanged,
                    actual_unchanged,
                    depth,
                    override_color,
                )?;
            }
            _ => cmp_fallback(f, expected, actual, depth, name, override_color)?,
        }
        Ok(())
    }
}

impl Compare for RelatedUnchangedDocumentDiagnosticReport {
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
        writeln!(
            f,
            "{padding}{name_str}RelatedUnchangedDocumentDiagnosticReport {{"
        )?;
        <Option<HashMap<Uri, DocumentDiagnosticReportKind>>>::compare(
            f,
            Some("related_documents"),
            &expected.related_documents,
            &actual.related_documents,
            depth + 1,
            override_color,
        )?;
        UnchangedDocumentDiagnosticReport::compare(
            f,
            Some("unchanged_document_diagnostic_report"),
            &expected.unchanged_document_diagnostic_report,
            &actual.unchanged_document_diagnostic_report,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;
        Ok(())
    }
}

impl Compare for RelatedFullDocumentDiagnosticReport {
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
        writeln!(
            f,
            "{padding}{name_str}RelatedFullDocumentDiagnosticReport {{"
        )?;
        <Option<HashMap<Uri, DocumentDiagnosticReportKind>>>::compare(
            f,
            Some("related_documents"),
            &expected.related_documents,
            &actual.related_documents,
            depth + 1,
            override_color,
        )?;
        FullDocumentDiagnosticReport::compare(
            f,
            Some("report"),
            &expected.full_document_diagnostic_report,
            &actual.full_document_diagnostic_report,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for DocumentDiagnosticReportKind {
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
        match (expected, actual) {
            (Self::Full(expected_full), Self::Full(actual_full)) => {
                writeln!(f, "{padding}{name_str}DocumentDiagnosticReportKind::Full (")?;
                FullDocumentDiagnosticReport::compare(
                    f,
                    name,
                    expected_full,
                    actual_full,
                    depth,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            (Self::Unchanged(expected_unchanged), Self::Unchanged(actual_unchanged)) => {
                writeln!(
                    f,
                    "{padding}{name_str}DocumentDiagnosticReportKind::Unchanged ("
                )?;
                UnchangedDocumentDiagnosticReport::compare(
                    f,
                    name,
                    expected_unchanged,
                    actual_unchanged,
                    depth,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            _ => cmp_fallback(f, expected, actual, depth, name, override_color)?,
        }
        Ok(())
    }
}

impl Compare for UnchangedDocumentDiagnosticReport {
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
        writeln!(f, "{padding}{name_str}UnchangedDocumentDiagnosticReport {{")?;
        String::compare(
            f,
            Some("result_id"),
            &expected.result_id,
            &actual.result_id,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;
        Ok(())
    }
}

impl Compare for FullDocumentDiagnosticReport {
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
        writeln!(f, "{padding}{name_str}FullDocumentDiagnosticReport {{")?;
        <Option<String>>::compare(
            f,
            Some("result_id"),
            &expected.result_id,
            &actual.result_id,
            depth + 1,
            override_color,
        )?;
        <Vec<Diagnostic>>::compare(
            f,
            Some("items"),
            &expected.items,
            &actual.items,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for Diagnostic {
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
        writeln!(f, "{padding}{name_str}Diagnostic {{")?;
        Range::compare(
            f,
            Some("range"),
            &expected.range,
            &actual.range,
            depth + 1,
            override_color,
        )?;
        <Option<DiagnosticSeverity>>::compare(
            f,
            Some("severity"),
            &expected.severity,
            &actual.severity,
            depth + 1,
            override_color,
        )?;
        <Option<NumberOrString>>::compare(
            f,
            Some("code"),
            &expected.code,
            &actual.code,
            depth + 1,
            override_color,
        )?;
        <Option<CodeDescription>>::compare(
            f,
            Some("code_description"),
            &expected.code_description,
            &actual.code_description,
            depth + 1,
            override_color,
        )?;
        <Option<String>>::compare(
            f,
            Some("source"),
            &expected.source,
            &actual.source,
            depth + 1,
            override_color,
        )?;
        String::compare(
            f,
            Some("message"),
            &expected.message,
            &actual.message,
            depth + 1,
            override_color,
        )?;
        <Option<Vec<DiagnosticRelatedInformation>>>::compare(
            f,
            Some("related_information"),
            &expected.related_information,
            &actual.related_information,
            depth + 1,
            override_color,
        )?;
        <Option<Vec<DiagnosticTag>>>::compare(
            f,
            Some("tags"),
            &expected.tags,
            &actual.tags,
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

impl Compare for DiagnosticSeverity {
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
        cmp_fallback(f, expected, actual, depth, name, override_color)
    }
}

impl Compare for NumberOrString {
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
        match (expected, actual) {
            (Self::Number(expected_num), Self::Number(actual_num)) => {
                i32::compare(f, name, expected_num, actual_num, depth, override_color)?;
            }
            (Self::String(expected_str), Self::String(actual_str)) => {
                String::compare(f, name, expected_str, actual_str, depth, override_color)?;
            }
            _ => cmp_fallback(f, expected, actual, depth, name, override_color)?,
        }
        Ok(())
    }
}

impl Compare for CodeDescription {
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
        writeln!(f, "{padding}{name_str}CodeDescription {{")?;
        Uri::compare(
            f,
            Some("href"),
            &expected.href,
            &actual.href,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for DiagnosticRelatedInformation {
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
        writeln!(f, "{padding}{name_str}DiagnosticRelatedInformation {{")?;
        Location::compare(
            f,
            Some("location"),
            &expected.location,
            &actual.location,
            depth + 1,
            override_color,
        )?;
        String::compare(
            f,
            Some("message"),
            &expected.message,
            &actual.message,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for DiagnosticTag {
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
        cmp_fallback(f, expected, actual, depth, name, override_color)
    }
}
