use lsp_types::{Moniker, MonikerKind, UniquenessLevel};
use thiserror::Error;

use super::{
    compare::{cmp_fallback, Compare},
    CleanResponse, Empty,
};

impl Empty for Vec<Moniker> {}
impl CleanResponse for Vec<Moniker> {}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct MonikerMismatchError {
    pub test_id: String,
    pub expected: Vec<Moniker>,
    pub actual: Vec<Moniker>,
}

impl std::fmt::Display for MonikerMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect Moniker response:", self.test_id)?;
        <Vec<Moniker>>::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

impl Compare for Moniker {
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
        writeln!(f, "{padding}{name_str}Moniker {{")?;
        String::compare(
            f,
            Some("scheme"),
            &expected.scheme,
            &actual.scheme,
            depth + 1,
            override_color,
        )?;
        String::compare(
            f,
            Some("identifier"),
            &expected.identifier,
            &actual.identifier,
            depth + 1,
            override_color,
        )?;
        UniquenessLevel::compare(
            f,
            Some("unique"),
            &expected.unique,
            &actual.unique,
            depth + 1,
            override_color,
        )?;
        <Option<MonikerKind>>::compare(
            f,
            Some("kind"),
            &expected.kind,
            &actual.kind,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for UniquenessLevel {
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

impl Compare for MonikerKind {
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
