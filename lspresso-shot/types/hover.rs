use lsp_types::{
    Hover, HoverContents, LanguageString, MarkedString, MarkupContent, MarkupKind, Range,
};
use thiserror::Error;

use super::{
    compare::{cmp_fallback, Compare},
    CleanResponse, Empty,
};

impl Empty for Hover {}

impl CleanResponse for Hover {}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct HoverMismatchError {
    pub test_id: String,
    pub expected: Hover,
    pub actual: Hover,
}

impl std::fmt::Display for HoverMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect Hover response:", self.test_id)?;
        Hover::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

impl Compare for Hover {
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
        writeln!(f, "{padding}{name_str}Hover {{")?;
        HoverContents::compare(
            f,
            Some("contents"),
            &expected.contents,
            &actual.contents,
            depth + 1,
            override_color,
        )?;
        Option::<Range>::compare(
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

impl Compare for HoverContents {
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
            (Self::Scalar(expected), Self::Scalar(actual)) => {
                writeln!(f, "{padding}{name_str}HoverContents::Scalar (")?;
                MarkedString::compare(
                    f,
                    Some("value"),
                    expected,
                    actual,
                    depth + 1,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            (Self::Array(expected), Self::Array(actual)) => {
                writeln!(f, "{padding}{name_str}HoverContents::Array (")?;
                <Vec<MarkedString>>::compare(
                    f,
                    Some("value"),
                    expected,
                    actual,
                    depth + 1,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            (Self::Markup(expected), Self::Markup(actual)) => {
                writeln!(f, "{padding}{name_str}HoverContents::MarkupContent (")?;
                MarkupContent::compare(f, None, expected, actual, depth + 1, override_color)?;
                writeln!(f, "{padding})")?;
            }
            _ => cmp_fallback(f, expected, actual, depth, name, override_color)?,
        }

        Ok(())
    }
}

impl Compare for MarkupKind {
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

impl Compare for MarkupContent {
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
        writeln!(f, "{padding}{name_str}MarkupContent {{")?;
        MarkupKind::compare(
            f,
            Some("kind"),
            &expected.kind,
            &actual.kind,
            depth + 1,
            override_color,
        )?;
        String::compare(
            f,
            Some("value"),
            &expected.value,
            &actual.value,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for MarkedString {
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
            (Self::String(expected), Self::String(actual)) => {
                writeln!(f, "{padding}{name_str}MarkedString::String (")?;
                String::compare(f, None, expected, actual, depth + 1, override_color)?;
                writeln!(f, "{padding})")?;
            }
            (Self::LanguageString(expected), Self::LanguageString(actual)) => {
                writeln!(f, "{padding}{name_str}MarkedString::LanguageString (")?;
                LanguageString::compare(f, None, expected, actual, depth + 1, override_color)?;
                writeln!(f, "{padding})")?;
            }
            _ => cmp_fallback(f, expected, actual, depth, name, override_color)?,
        }

        Ok(())
    }
}

impl Compare for LanguageString {
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
        writeln!(f, "{padding}{name_str}LanguageString {{")?;
        String::compare(
            f,
            Some("language"),
            &expected.language,
            &actual.language,
            depth + 1,
            override_color,
        )?;
        String::compare(
            f,
            Some("value"),
            &expected.value,
            &actual.value,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}
