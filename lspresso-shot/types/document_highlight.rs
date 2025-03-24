use lsp_types::{DocumentHighlight, DocumentHighlightKind, Range};
use thiserror::Error;

use super::{
    compare::{cmp_fallback, Compare},
    CleanResponse, Empty,
};

impl Empty for Vec<DocumentHighlight> {}

impl CleanResponse for Vec<DocumentHighlight> {}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct DocumentHighlightMismatchError {
    pub test_id: String,
    pub expected: Vec<DocumentHighlight>,
    pub actual: Vec<DocumentHighlight>,
}

impl std::fmt::Display for DocumentHighlightMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Document Highlight response:",
            self.test_id
        )?;
        <Vec<DocumentHighlight>>::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

impl Compare for DocumentHighlight {
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
        writeln!(f, "{padding}{name_str}WorkspaceEdit {{")?;
        Range::compare(
            f,
            name,
            &expected.range,
            &actual.range,
            depth,
            override_color,
        )?;
        <Option<DocumentHighlightKind>>::compare(
            f,
            Some("kind"),
            &expected.kind,
            &actual.kind,
            depth,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for DocumentHighlightKind {
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
