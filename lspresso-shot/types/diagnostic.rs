use std::collections::HashMap;

use lsp_types::{
    Diagnostic, DocumentDiagnosticReport, DocumentDiagnosticReportKind, WorkspaceDiagnosticReport,
    WorkspaceDocumentDiagnosticReport,
};
use thiserror::Error;

use super::{
    CleanResponse, Empty, TestCase, TestResult, clean_uri, compare::write_fields_comparison,
};

impl Empty for Vec<Diagnostic> {}
impl Empty for DocumentDiagnosticReport {}
impl Empty for WorkspaceDiagnosticReport {}

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

impl CleanResponse for DocumentDiagnosticReportKind {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        match &mut self {
            Self::Full(report) => {
                report.items = report.items.clone().clean_response(test_case)?;
            }
            Self::Unchanged(_) => {}
        }

        Ok(self)
    }
}

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

impl CleanResponse for WorkspaceDiagnosticReport {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        for report in &mut self.items {
            match report {
                WorkspaceDocumentDiagnosticReport::Full(report) => {
                    report.uri = clean_uri(&report.uri, test_case)?;
                    for report in &mut report.full_document_diagnostic_report.items {
                        if let Some(ref mut related_info) = report.related_information {
                            for info in related_info {
                                info.location.uri = clean_uri(&info.location.uri, test_case)?;
                            }
                        }
                    }
                }
                WorkspaceDocumentDiagnosticReport::Unchanged(report) => {
                    report.uri = clean_uri(&report.uri, test_case)?;
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
        write_fields_comparison(f, "Vec<Diagnostic>", &self.expected, &self.actual, 0)
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
        write_fields_comparison(
            f,
            "DocumentDiagnosticReport",
            &self.expected,
            &self.actual,
            0,
        )
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct WorkspaceDiagnosticMismatchError {
    pub test_id: String,
    pub expected: WorkspaceDiagnosticReport,
    pub actual: WorkspaceDiagnosticReport,
}

impl std::fmt::Display for WorkspaceDiagnosticMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Workspace Diagnostic response:",
            self.test_id
        )?;
        write_fields_comparison(
            f,
            "WorkspaceDiagnosticReport",
            &self.expected,
            &self.actual,
            0,
        )
    }
}
