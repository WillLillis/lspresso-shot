use lsp_types::{CodeAction, CodeActionOrCommand, CodeActionResponse};

use super::{CleanResponse, TestExecutionResult};

impl CleanResponse for CodeActionResponse {
    fn clean_response(mut self, test_case: &super::TestCase) -> TestExecutionResult<Self> {
        for action in &mut self {
            match action {
                CodeActionOrCommand::Command(_) => {}
                CodeActionOrCommand::CodeAction(action) => {
                    if let Some(ref mut edit) = action.edit {
                        *edit = edit.clone().clean_response(test_case)?;
                    }
                }
            }
        }
        Ok(self)
    }
}

impl CleanResponse for CodeAction {
    fn clean_response(mut self, test_case: &super::TestCase) -> TestExecutionResult<Self> {
        if let Some(ref mut edit) = self.edit {
            *edit = edit.clone().clean_response(test_case)?;
        }
        Ok(self)
    }
}
