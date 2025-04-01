use lsp_types::{request::GotoTypeDefinitionResponse, Location, LocationLink, Range, Uri};
use thiserror::Error;

use super::compare::{cmp_fallback, Compare};

#[derive(Debug, Error, PartialEq, Eq)]
pub struct TypeDefinitionMismatchError {
    pub test_id: String,
    pub expected: GotoTypeDefinitionResponse,
    pub actual: GotoTypeDefinitionResponse,
}

impl Compare for Location {
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
        writeln!(f, "{padding}{name_str} Location {{")?;
        Uri::compare(
            f,
            Some("uri"),
            &expected.uri,
            &actual.uri,
            depth + 1,
            override_color,
        )?;
        Range::compare(
            f,
            Some("range"),
            &expected.range,
            &actual.range,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for LocationLink {
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
        writeln!(f, "{padding}{name_str} LocationLink {{")?;
        Option::<Range>::compare(
            f,
            Some("origin_selection_range"),
            &expected.origin_selection_range,
            &actual.origin_selection_range,
            depth + 1,
            override_color,
        )?;
        Uri::compare(
            f,
            Some("target_uri"),
            &expected.target_uri,
            &actual.target_uri,
            depth + 1,
            override_color,
        )?;
        Range::compare(
            f,
            Some("target_range"),
            &expected.target_range,
            &actual.target_range,
            depth + 1,
            override_color,
        )?;
        Range::compare(
            f,
            Some("target_selection_range"),
            &expected.target_selection_range,
            &actual.target_selection_range,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for GotoTypeDefinitionResponse {
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
            (Self::Scalar(expected_loc), Self::Scalar(actual_loc)) => {
                writeln!(f, "{padding}{name_str}GotoTypeDefinitionResponse::Scalar (")?;
                Location::compare(f, None, expected_loc, actual_loc, depth + 1, override_color)?;
                writeln!(f, "{padding})")?;
            }
            (Self::Array(expected_arr), Self::Array(actual_arr)) => {
                writeln!(f, "{padding}{name_str}GotoTypeDefinitionResponse::Array (")?;
                Vec::compare(f, None, expected_arr, actual_arr, depth + 1, override_color)?;
                writeln!(f, "{padding})")?;
            }
            (Self::Link(expected_links), Self::Link(actual_links)) => {
                writeln!(f, "{padding}{name_str}GotoTypeDefinitionResponse::Link (")?;
                Vec::compare(
                    f,
                    None,
                    expected_links,
                    actual_links,
                    depth + 1,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            _ => cmp_fallback(f, expected, actual, depth, name, override_color)?,
        }
        writeln!(f, "{padding})")?;

        Ok(())
    }
}

impl std::fmt::Display for TypeDefinitionMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect GotoTypeDefinition response:",
            self.test_id
        )?;
        GotoTypeDefinitionResponse::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}
