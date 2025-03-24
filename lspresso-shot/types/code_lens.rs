use lsp_types::{CodeLens, Command, Range};
use thiserror::Error;

use super::{compare::Compare, CleanResponse, Empty};

impl Empty for CodeLens {}
impl Empty for Vec<CodeLens> {}

impl CleanResponse for CodeLens {}
impl CleanResponse for Vec<CodeLens> {}

#[derive(Debug, Error, PartialEq)]
pub struct CodeLensMismatchError {
    pub test_id: String,
    pub expected: Vec<CodeLens>,
    pub actual: Vec<CodeLens>,
}

impl std::fmt::Display for CodeLensMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect CodeLens response:", self.test_id)?;
        <Vec<CodeLens>>::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

#[derive(Debug, Error, PartialEq)]
pub struct CodeLensResolveMismatchError {
    pub test_id: String,
    pub expected: CodeLens,
    pub actual: CodeLens,
}

impl std::fmt::Display for CodeLensResolveMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect CodeLens Resolve response:",
            self.test_id
        )?;
        CodeLens::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

impl Compare for CodeLens {
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
        writeln!(f, "{padding}{name_str}CodeLens {{")?;
        Range::compare(
            f,
            Some("range"),
            &expected.range,
            &actual.range,
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
        <Option<serde_json::Value>>::compare(
            f,
            Some("data"),
            &expected.data,
            &actual.data,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}", padding = "  ".repeat(depth))?;

        Ok(())
    }
}
