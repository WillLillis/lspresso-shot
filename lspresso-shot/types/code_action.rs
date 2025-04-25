use lsp_types::{
    CodeAction, CodeActionDisabled, CodeActionKind, CodeActionOrCommand, CodeActionResponse,
    Command, Diagnostic, WorkspaceEdit,
};
use serde_json::Value;
use thiserror::Error;

use super::{
    compare::{cmp_fallback, Compare},
    CleanResponse, Empty, TestResult,
};

impl Empty for CodeActionResponse {}

impl CleanResponse for CodeActionResponse {
    fn clean_response(mut self, test_case: &super::TestCase) -> TestResult<Self> {
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

#[derive(Debug, Error, PartialEq, Eq)]
pub struct CodeActionMismatchError {
    pub test_id: String,
    pub expected: CodeActionResponse,
    pub actual: CodeActionResponse,
}

impl std::fmt::Display for CodeActionMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect Code Action response:", self.test_id)?;
        CodeActionResponse::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

impl Compare for CodeActionOrCommand {
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
            (Self::Command(expected_cmd), Self::Command(actual_cmd)) => {
                writeln!(f, "{padding}{name_str} CodeActionOrCommand::Command (")?;
                Command::compare(f, None, expected_cmd, actual_cmd, depth + 1, override_color)?;
                writeln!(f, "{padding})")?;
            }
            (Self::CodeAction(expected_action), Self::CodeAction(actual_action)) => {
                writeln!(f, "{padding}{name_str} CodeActionOrCommand::CodeAction (")?;
                CodeAction::compare(
                    f,
                    None,
                    expected_action,
                    actual_action,
                    depth + 1,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            _ => cmp_fallback(f, expected, actual, depth, name, override_color)?,
        }
        Ok(())
    }
}

impl Compare for CodeAction {
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
        writeln!(f, "{padding}{name_str}CodeAction {{")?;
        String::compare(
            f,
            Some("title"),
            &expected.title,
            &actual.title,
            depth + 1,
            override_color,
        )?;
        <Option<CodeActionKind>>::compare(
            f,
            Some("kind"),
            &expected.kind,
            &actual.kind,
            depth + 1,
            override_color,
        )?;
        <Option<Vec<Diagnostic>>>::compare(
            f,
            Some("diagnostics"),
            &expected.diagnostics,
            &actual.diagnostics,
            depth + 1,
            override_color,
        )?;
        <Option<WorkspaceEdit>>::compare(
            f,
            Some("edit"),
            &expected.edit,
            &actual.edit,
            depth + 1,
            override_color,
        )?;
        <Option<Command>>::compare(
            f,
            Some("command"),
            &expected.command,
            &actual.command,
            depth + 1,
            override_color,
        )?;
        <Option<bool>>::compare(
            f,
            Some("is_preferred"),
            &expected.is_preferred,
            &actual.is_preferred,
            depth + 1,
            override_color,
        )?;
        <Option<CodeActionDisabled>>::compare(
            f,
            Some("disabled"),
            &expected.disabled,
            &actual.disabled,
            depth + 1,
            override_color,
        )?;
        <Option<Value>>::compare(
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

impl Compare for CodeActionKind {
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

impl Compare for CodeActionDisabled {
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
        writeln!(f, "{padding}{name_str}CodeActionDisabled {{")?;
        String::compare(
            f,
            Some("reason"),
            &expected.reason,
            &actual.reason,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;
        Ok(())
    }
}
