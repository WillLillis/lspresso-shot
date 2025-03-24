use lsp_types::{Range, SelectionRange};
use thiserror::Error;

use super::{compare::Compare, CleanResponse, Empty};

impl Empty for Vec<SelectionRange> {}

impl CleanResponse for Vec<SelectionRange> {}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct SelectionRangeMismatchError {
    pub test_id: String,
    pub expected: Vec<SelectionRange>,
    pub actual: Vec<SelectionRange>,
}

impl std::fmt::Display for SelectionRangeMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Selection Range response:",
            self.test_id
        )?;
        <Vec<SelectionRange>>::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

impl Compare for SelectionRange {
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
        writeln!(f, "{padding}{name_str}SelectionRange {{")?;
        Range::compare(
            f,
            Some("range"),
            &expected.range,
            &actual.range,
            depth + 1,
            override_color,
        )?;
        Option::<Box<Self>>::compare(
            f,
            Some("parent"),
            &expected.parent,
            &actual.parent,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for Box<SelectionRange> {
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
        let expected = *(expected.clone());
        let actual = *(actual.clone());
        SelectionRange::compare(f, name, &expected, &actual, depth, override_color)
    }
}
