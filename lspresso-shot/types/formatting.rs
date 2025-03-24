use lsp_types::{AnnotatedTextEdit, ChangeAnnotationIdentifier, Range, TextEdit};
use thiserror::Error;

use super::{compare::Compare, CleanResponse, Empty};

impl Empty for FormattingResult {}
impl Empty for Vec<TextEdit> {}

impl CleanResponse for FormattingResult {}
impl CleanResponse for Vec<TextEdit> {}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FormattingResult {
    /// Check if the file's formatted state matches the expected contents
    EndState(String),
    /// Check if the server's response matches the exected edits
    Response(Vec<TextEdit>),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct FormattingMismatchError {
    pub test_id: String,
    pub expected: FormattingResult,
    pub actual: FormattingResult,
}

impl std::fmt::Display for FormattingMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect Formatting response:", self.test_id)?;
        FormattingResult::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

impl Compare for FormattingResult {
    type Nested1 = ();
    type Nested2 = TextEdit;
    fn compare(
        f: &mut std::fmt::Formatter<'_>,
        _name: Option<&str>,
        expected: &Self,
        actual: &Self,
        depth: usize,
        override_color: Option<anstyle::Color>,
    ) -> std::fmt::Result {
        let padding = "  ".repeat(depth);
        match (expected, actual) {
            (Self::EndState(expected_end_state), Self::EndState(actual_end_state)) => {
                writeln!(f, "{padding}FormattingResult::EndState (")?;
                String::compare(
                    f,
                    None,
                    expected_end_state,
                    actual_end_state,
                    depth + 1,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            (Self::Response(expected_edits), Self::Response(actual_edits)) => {
                writeln!(f, "{padding}FormattingResult::Response (")?;
                <Vec<TextEdit>>::compare(
                    f,
                    None,
                    expected_edits,
                    actual_edits,
                    depth + 1,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}

impl Compare for AnnotatedTextEdit {
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
        writeln!(f, "{padding}{name_str}AnnotatedTextEdit {{")?;
        TextEdit::compare(
            f,
            Some("text_edit"),
            &expected.text_edit,
            &actual.text_edit,
            depth + 1,
            override_color,
        )?;
        ChangeAnnotationIdentifier::compare(
            f,
            Some("annotation_id"),
            &expected.annotation_id,
            &actual.annotation_id,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for TextEdit {
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
        writeln!(f, "{padding}{name_str}TextEdit {{")?;
        Range::compare(
            f,
            Some("range"),
            &expected.range,
            &actual.range,
            depth + 1,
            override_color,
        )?;
        String::compare(
            f,
            Some("new_text"),
            &expected.new_text,
            &actual.new_text,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}
