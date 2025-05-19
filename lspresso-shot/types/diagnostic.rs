use std::collections::HashMap;

use lsp_types::{
    Diagnostic, DocumentDiagnosticReport, DocumentDiagnosticReportKind, WorkspaceDiagnosticReport,
    WorkspaceDocumentDiagnosticReport,
};

use super::{ApproximateEq, CleanResponse, TestCase, TestExecutionResult, clean_uri};

impl CleanResponse for Vec<Diagnostic> {
    fn clean_response(mut self, test_case: &TestCase) -> TestExecutionResult<Self> {
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
    fn clean_response(mut self, test_case: &TestCase) -> TestExecutionResult<Self> {
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
    fn clean_response(mut self, test_case: &TestCase) -> TestExecutionResult<Self> {
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
    fn clean_response(mut self, test_case: &TestCase) -> TestExecutionResult<Self> {
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

impl ApproximateEq for DocumentDiagnosticReport {}
impl ApproximateEq for Vec<Diagnostic> {}
impl ApproximateEq for WorkspaceDiagnosticReport {}
