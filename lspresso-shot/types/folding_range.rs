use lsp_types::{FoldingRange, FoldingRangeKind};
use thiserror::Error;

use super::{
    compare::{cmp_fallback, Compare},
    CleanResponse, Empty,
};

impl Empty for Vec<FoldingRange> {}

impl CleanResponse for Vec<FoldingRange> {}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct FoldingRangeMismatchError {
    pub test_id: String,
    pub expected: Vec<FoldingRange>,
    pub actual: Vec<FoldingRange>,
}

impl std::fmt::Display for FoldingRangeMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Folding Range response:", self.test_id)?;
        <Vec<FoldingRange>>::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

impl Compare for FoldingRange {
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
        writeln!(f, "{padding}{name_str}FoldingRange {{")?;
        u32::compare(
            f,
            Some("start_line"),
            &expected.start_line,
            &actual.start_line,
            depth + 1,
            override_color,
        )?;
        <Option<u32>>::compare(
            f,
            Some("start_character"),
            &expected.start_character,
            &actual.start_character,
            depth + 1,
            override_color,
        )?;
        u32::compare(
            f,
            Some("end_line"),
            &expected.end_line,
            &actual.end_line,
            depth + 1,
            override_color,
        )?;
        <Option<u32>>::compare(
            f,
            Some("end_character"),
            &expected.end_character,
            &actual.end_character,
            depth + 1,
            override_color,
        )?;
        <Option<FoldingRangeKind>>::compare(
            f,
            Some("kind"),
            &expected.kind,
            &actual.kind,
            depth + 1,
            override_color,
        )?;
        <Option<String>>::compare(
            f,
            Some("collapse_text"),
            &expected.collapsed_text,
            &actual.collapsed_text,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for FoldingRangeKind {
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
