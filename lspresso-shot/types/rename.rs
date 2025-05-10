use std::collections::HashMap;

use lsp_types::{DocumentChangeOperation, DocumentChanges, ResourceOp, WorkspaceEdit};
use thiserror::Error;

use super::{
    CleanResponse, Empty, TestCase, TestResult, clean_uri, compare::write_fields_comparison,
};

impl Empty for WorkspaceEdit {}

impl CleanResponse for WorkspaceEdit {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        if let Some(ref mut changes) = self.changes {
            let mut new_changes = HashMap::new();
            for (uri, edits) in changes.drain() {
                let cleaned_uri = clean_uri(&uri, test_case)?;
                new_changes.insert(cleaned_uri, edits);
            }
            *changes = new_changes;
        }
        match self.document_changes {
            Some(DocumentChanges::Edits(ref mut edits)) => {
                for edit in edits {
                    edit.text_document.uri = clean_uri(&edit.text_document.uri, test_case)?;
                }
            }
            Some(DocumentChanges::Operations(ref mut ops)) => {
                for op in ops {
                    match op {
                        DocumentChangeOperation::Op(op) => match op {
                            ResourceOp::Create(create) => {
                                create.uri = clean_uri(&create.uri, test_case)?;
                            }
                            ResourceOp::Rename(rename) => {
                                rename.old_uri = clean_uri(&rename.old_uri, test_case)?;
                                rename.new_uri = clean_uri(&rename.new_uri, test_case)?;
                            }
                            ResourceOp::Delete(delete) => {
                                delete.uri = clean_uri(&delete.uri, test_case)?;
                            }
                        },
                        DocumentChangeOperation::Edit(edit) => {
                            edit.text_document.uri = clean_uri(&edit.text_document.uri, test_case)?;
                        }
                    }
                }
            }
            None => {}
        }
        Ok(self)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct RenameMismatchError {
    pub test_id: String,
    pub expected: WorkspaceEdit,
    pub actual: WorkspaceEdit,
}

impl std::fmt::Display for RenameMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect Rename response:", self.test_id)?;
        write_fields_comparison(f, "WorkspaceEdit", &self.expected, &self.actual, 0)
    }
}
