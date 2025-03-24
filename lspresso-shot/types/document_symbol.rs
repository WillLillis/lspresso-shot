use lsp_types::{
    DocumentSymbol, DocumentSymbolResponse, Location, Range, SymbolInformation, SymbolKind,
    SymbolTag,
};
use thiserror::Error;

use super::{
    clean_uri,
    compare::{cmp_fallback, paint, Compare, RED},
    CleanResponse, Empty, TestCase, TestResult,
};

impl Empty for DocumentSymbolResponse {}

impl CleanResponse for DocumentSymbolResponse {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        match &mut self {
            Self::Flat(syms) => {
                for sym in syms {
                    sym.location.uri = clean_uri(&sym.location.uri, test_case)?;
                }
            }
            Self::Nested(_) => {}
        }
        Ok(self)
    }
}

#[derive(Debug, Error, PartialEq)]
pub struct DocumentSymbolMismatchError {
    pub test_id: String,
    pub expected: DocumentSymbolResponse,
    pub actual: DocumentSymbolResponse,
}

impl std::fmt::Display for DocumentSymbolMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Document Symbol response:",
            self.test_id
        )?;
        DocumentSymbolResponse::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

impl Compare for DocumentSymbolResponse {
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
            (Self::Flat(expected), Self::Flat(actual)) => {
                writeln!(f, "{padding}{name_str}DocumentSymbolResponse::Flat (")?;
                <Vec<SymbolInformation>>::compare(
                    f,
                    None,
                    expected,
                    actual,
                    depth + 1,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            (Self::Nested(expected), Self::Nested(actual)) => {
                writeln!(f, "{padding}{name_str}DocumentSymbolResponse::Nested (")?;
                <Vec<DocumentSymbol>>::compare(
                    f,
                    None,
                    expected,
                    actual,
                    depth + 1,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            _ => {
                writeln!(
                    f,
                    "{padding}{name_str}{}",
                    paint(
                        override_color.unwrap_or(RED.unwrap()).into(),
                        &format!(
                            "{padding}  Expected:\n{padding}    {expected:?}\n{padding}  Actual:\n{padding}    {actual:?}"
                        )
                    )
                )?;
            }
        }

        Ok(())
    }
}

impl Compare for DocumentSymbol {
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
        writeln!(f, "{padding}{name_str}DocumentSymbol {{")?;
        String::compare(
            f,
            Some("name"),
            &expected.name,
            &actual.name,
            depth + 1,
            override_color,
        )?;
        <Option<String>>::compare(
            f,
            Some("detail"),
            &expected.detail,
            &actual.detail,
            depth + 1,
            override_color,
        )?;
        <Option<Vec<SymbolTag>>>::compare(
            f,
            Some("tags"),
            &expected.tags,
            &actual.tags,
            depth + 1,
            override_color,
        )?;
        Option::compare(
            f,
            Some("deprecated"),
            #[allow(deprecated)]
            &expected.deprecated,
            #[allow(deprecated)]
            &actual.deprecated,
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
        Range::compare(
            f,
            Some("selection_range"),
            &expected.selection_range,
            &actual.selection_range,
            depth + 1,
            override_color,
        )?;
        <Option<Vec<Self>>>::compare(
            f,
            Some("children"),
            &expected.children,
            &actual.children,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for SymbolInformation {
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
        writeln!(f, "{padding}{name_str}SymbolInformation {{")?;
        String::compare(
            f,
            Some("name"),
            &expected.name,
            &actual.name,
            depth + 1,
            override_color,
        )?;
        SymbolKind::compare(
            f,
            Some("kind"),
            &expected.kind,
            &actual.kind,
            depth + 1,
            override_color,
        )?;
        Option::compare(
            f,
            Some("tags"),
            &expected.tags,
            &actual.tags,
            depth + 1,
            override_color,
        )?;
        <Option<bool>>::compare(
            f,
            Some("deprecated"),
            #[allow(deprecated)]
            &expected.deprecated,
            #[allow(deprecated)]
            &actual.deprecated,
            depth + 1,
            override_color,
        )?;
        Location::compare(
            f,
            Some("location"),
            &expected.location,
            &actual.location,
            depth + 1,
            override_color,
        )?;
        <Option<String>>::compare(
            f,
            Some("container_name"),
            &expected.container_name,
            &actual.container_name,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for SymbolKind {
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

impl Compare for SymbolTag {
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
