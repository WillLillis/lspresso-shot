use lsp_types::{
    SemanticToken, SemanticTokens, SemanticTokensDelta, SemanticTokensEdit,
    SemanticTokensFullDeltaResult, SemanticTokensPartialResult, SemanticTokensRangeResult,
    SemanticTokensResult,
};
use thiserror::Error;

use super::{
    compare::{cmp_fallback, Compare},
    CleanResponse, Empty,
};

impl Empty for SemanticTokensResult {}
impl Empty for SemanticTokensFullDeltaResult {}
impl Empty for SemanticTokensRangeResult {}

impl CleanResponse for SemanticTokensResult {}
impl CleanResponse for SemanticTokensFullDeltaResult {}
impl CleanResponse for SemanticTokensRangeResult {}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct SemanticTokensFullMismatchError {
    pub test_id: String,
    pub expected: SemanticTokensResult,
    pub actual: SemanticTokensResult,
}

impl std::fmt::Display for SemanticTokensFullMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Semantic Tokens Full response:",
            self.test_id
        )?;
        SemanticTokensResult::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct SemanticTokensFullDeltaMismatchError {
    pub test_id: String,
    pub expected: SemanticTokensFullDeltaResult,
    pub actual: SemanticTokensFullDeltaResult,
}

impl std::fmt::Display for SemanticTokensFullDeltaMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Semantic Tokens Full Delta response:",
            self.test_id
        )?;
        SemanticTokensFullDeltaResult::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct SemanticTokensRangeMismatchError {
    pub test_id: String,
    pub expected: SemanticTokensRangeResult,
    pub actual: SemanticTokensRangeResult,
}

impl std::fmt::Display for SemanticTokensRangeMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Semantic Tokens Range response:",
            self.test_id
        )?;
        SemanticTokensRangeResult::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

impl Compare for SemanticTokensRangeResult {
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
            (Self::Tokens(expected_tokens), Self::Tokens(actual_tokens)) => {
                writeln!(f, "{padding}{name_str}SemanticTokensRangeResult::Tokens (")?;
                SemanticTokens::compare(
                    f,
                    None,
                    expected_tokens,
                    actual_tokens,
                    depth + 1,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            (Self::Partial(expected_partial_result), Self::Partial(actual_partial_result)) => {
                writeln!(f, "{padding}{name_str}SemanticTokensRangeResult::Tokens (")?;
                SemanticTokensPartialResult::compare(
                    f,
                    None,
                    expected_partial_result,
                    actual_partial_result,
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

impl Compare for SemanticTokensFullDeltaResult {
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
            (Self::Tokens(expected_tokens), Self::Tokens(actual_tokens)) => {
                writeln!(
                    f,
                    "{padding}{name_str}SemanticTokensFullDeltaResult::Tokens ("
                )?;
                SemanticTokens::compare(
                    f,
                    name,
                    expected_tokens,
                    actual_tokens,
                    depth,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            (Self::TokensDelta(expected_delta), Self::TokensDelta(actual_delta)) => {
                writeln!(
                    f,
                    "{padding}{name_str}SemanticTokensFullDeltaResult::TokensDelta ("
                )?;
                SemanticTokensDelta::compare(
                    f,
                    name,
                    expected_delta,
                    actual_delta,
                    depth,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            (
                Self::PartialTokensDelta {
                    edits: expected_edits,
                },
                Self::PartialTokensDelta {
                    edits: actual_edits,
                },
            ) => {
                writeln!(
                    f,
                    "{padding}{name_str}SemanticTokensFullDeltaResult::PartialTokensDelta ("
                )?;
                <Vec<SemanticTokensEdit>>::compare(
                    f,
                    name,
                    expected_edits,
                    actual_edits,
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

impl Compare for SemanticTokensDelta {
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
        writeln!(f, "{padding}{name_str}SemanticTokensDelta {{")?;
        <Option<String>>::compare(
            f,
            Some("result_id"),
            &expected.result_id,
            &actual.result_id,
            depth + 1,
            override_color,
        )?;
        <Vec<SemanticTokensEdit>>::compare(
            f,
            Some("edits"),
            &expected.edits,
            &actual.edits,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for SemanticTokensResult {
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
            (Self::Tokens(expected_tokens), Self::Tokens(actual_tokens)) => {
                writeln!(f, "{padding}{name_str}SemanticTokensResult::Tokens (")?;
                SemanticTokens::compare(
                    f,
                    None,
                    expected_tokens,
                    actual_tokens,
                    depth + 1,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            (Self::Partial(expected_full), Self::Partial(actual_full)) => {
                writeln!(f, "{padding}{name_str}SemanticTokensResult::Partial (")?;
                SemanticTokensPartialResult::compare(
                    f,
                    None,
                    expected_full,
                    actual_full,
                    depth + 1,
                    override_color,
                )?;
            }
            _ => cmp_fallback(f, expected, actual, depth, name, override_color)?,
        }
        writeln!(f, "{padding})")?;

        Ok(())
    }
}

impl Compare for SemanticToken {
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
        writeln!(f, "{padding}{name_str}SemanticToken {{")?;
        u32::compare(
            f,
            Some("delta_line"),
            &expected.delta_line,
            &actual.delta_line,
            depth + 1,
            override_color,
        )?;
        u32::compare(
            f,
            Some("delta_start"),
            &expected.delta_start,
            &actual.delta_start,
            depth + 1,
            override_color,
        )?;
        u32::compare(
            f,
            Some("length"),
            &expected.length,
            &actual.length,
            depth + 1,
            override_color,
        )?;
        u32::compare(
            f,
            Some("token_type"),
            &expected.token_type,
            &actual.token_type,
            depth + 1,
            override_color,
        )?;
        u32::compare(
            f,
            Some("token_modifiers_bitset"),
            &expected.token_modifiers_bitset,
            &actual.token_modifiers_bitset,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for SemanticTokensEdit {
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
        writeln!(f, "{padding}{name_str}SemanticTokensEdit {{")?;
        u32::compare(
            f,
            Some("start"),
            &expected.start,
            &actual.start,
            depth + 1,
            override_color,
        )?;
        u32::compare(
            f,
            Some("delete_count"),
            &expected.delete_count,
            &actual.delete_count,
            depth + 1,
            override_color,
        )?;
        <Option<Vec<SemanticToken>>>::compare(
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

impl Compare for SemanticTokensPartialResult {
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
        writeln!(f, "{padding}{name_str}SemanticTokensPartialResult {{")?;
        <Vec<SemanticToken>>::compare(
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

impl Compare for SemanticTokens {
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
        writeln!(f, "{padding}{name_str}SemanticTokens {{")?;
        <Option<String>>::compare(
            f,
            Some("result_id"),
            &expected.result_id,
            &actual.result_id,
            depth + 1,
            override_color,
        )?;
        <Vec<SemanticToken>>::compare(
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
