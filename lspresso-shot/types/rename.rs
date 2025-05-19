use std::collections::HashMap;

use lsp_types::{
    DocumentChangeOperation, DocumentChanges, PrepareRenameResponse, ResourceOp, WorkspaceEdit,
};

use super::{ApproximateEq, CleanResponse, TestCase, TestExecutionResult, clean_uri};

impl CleanResponse for WorkspaceEdit {
    fn clean_response(mut self, test_case: &TestCase) -> TestExecutionResult<Self> {
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

impl CleanResponse for PrepareRenameResponse {}

impl ApproximateEq for PrepareRenameResponse {}
impl ApproximateEq for WorkspaceEdit {}
