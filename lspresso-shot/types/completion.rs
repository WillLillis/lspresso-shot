use lsp_types::{
    Command, CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionItemTag,
    CompletionList, CompletionResponse, CompletionTextEdit, Documentation, InsertReplaceEdit,
    InsertTextFormat, InsertTextMode, MarkupContent, Range, TextEdit,
};
use thiserror::Error;

use super::{
    compare::{cmp_fallback, paint, Compare, RED},
    CleanResponse, Empty,
};

impl Empty for CompletionResponse {}

impl CleanResponse for CompletionResponse {}

// `textDocument/completion` is a bit different from other requests. Servers commonly
// send a *bunch* of completion items and rely on the editor's lsp client to filter
// them out/ display the most relevant ones first. This is fine, but it means that
// doing a simple equality check for this isn't realistic and would be a serious
// pain for library consumers. I'd like to experiment with the different ways we
// can handle this, but for now we'll just allow for exact matching, and a simple
// "contains" check.
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionResult {
    /// Expect this exact set of completion items in the provided order
    Exact(CompletionResponse),
    /// Expect to at least see these completion items in any order.
    /// NOTE: This discards the `CompletionList.is_incomplete` field and only
    /// considers `CompletionList.items`
    Contains(Vec<CompletionItem>),
}

impl CompletionResult {
    /// Compares the expected results in `self` to the `actual` results, respecting
    /// the intended behavior for each enum variant of `Self`
    ///
    /// Returns true if the two are considered equal, false otherwise
    #[must_use]
    pub fn results_satisfy(&self, actual: &CompletionResponse) -> bool {
        match self {
            Self::Contains(expected_results) => {
                let actual_items = match actual {
                    CompletionResponse::Array(a) => a,
                    CompletionResponse::List(CompletionList { items, .. }) => items,
                };
                let mut expected = expected_results.clone();
                for item in actual_items {
                    if let Some(i) = expected
                        .iter()
                        .enumerate()
                        .find(|(_, e)| *e == item)
                        .map(|(i, _)| i)
                    {
                        expected.remove(i);
                    }
                }

                expected.is_empty()
            }
            Self::Exact(expected_results) => match (expected_results, actual) {
                (CompletionResponse::Array(expected), CompletionResponse::Array(actual)) => {
                    expected == actual
                }
                (
                    CompletionResponse::List(CompletionList {
                        is_incomplete: expected_is_incomplete,
                        items: expected_items,
                    }),
                    CompletionResponse::List(CompletionList {
                        is_incomplete: actual_is_incomplete,
                        items: actual_items,
                    }),
                ) => {
                    expected_is_incomplete == actual_is_incomplete && expected_items == actual_items
                }
                (CompletionResponse::Array(_), CompletionResponse::List(_))
                | (CompletionResponse::List(_), CompletionResponse::Array(_)) => false,
            },
        }
    }
}

#[derive(Debug, Error, PartialEq)]
pub struct CompletionMismatchError {
    pub test_id: String,
    pub expected: CompletionResult,
    pub actual: CompletionResponse,
}

// TODO: Cleanup/ consolidate this logic with Self::compare_results
impl std::fmt::Display for CompletionMismatchError {
    #[allow(clippy::too_many_lines)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.expected {
            CompletionResult::Contains(expected_results) => {
                let actual_items = match &self.actual {
                    CompletionResponse::Array(a) => a,
                    CompletionResponse::List(CompletionList { items, .. }) => items,
                };
                let mut expected = expected_results.clone();
                for item in actual_items {
                    if let Some(i) = expected
                        .iter()
                        .enumerate()
                        .find(|(_, e)| **e == *item)
                        .map(|(i, _)| i)
                    {
                        expected.remove(i);
                    }
                }

                writeln!(
                    f,
                    "Unprovided item{}:",
                    if expected.len() > 1 { "s" } else { "" }
                )?;
                for item in &expected {
                    writeln!(
                        f,
                        "{}",
                        paint(RED, &format!("{}", serde_json::to_value(item).unwrap()))
                    )?;
                }
                writeln!(
                    f,
                    "\nProvided item{}:",
                    if actual_items.len() > 1 { "s" } else { "" }
                )?;
                for item in actual_items {
                    writeln!(
                        f,
                        "{}",
                        paint(RED, &format!("{}", serde_json::to_value(item).unwrap()))
                    )?;
                }
            }
            CompletionResult::Exact(expected_results) => match (expected_results, &self.actual) {
                (CompletionResponse::Array(_), CompletionResponse::Array(_))
                | (CompletionResponse::List(_), CompletionResponse::List(_)) => {
                    <CompletionResponse>::compare(
                        f,
                        None,
                        expected_results,
                        &self.actual,
                        0,
                        None,
                    )?;
                }
                // Different completion types, indicate so and compare the inner items
                (
                    CompletionResponse::Array(expected_items),
                    CompletionResponse::List(CompletionList {
                        items: actual_items,
                        ..
                    }),
                ) => {
                    writeln!(
                        f,
                        "Expected `CompletionResponse::Array`, got `CompletionResponse::List`"
                    )?;
                    <Vec<CompletionItem>>::compare(
                        f,
                        Some("CompletionResponse"),
                        expected_items,
                        actual_items,
                        0,
                        None,
                    )?;
                }
                (
                    CompletionResponse::List(CompletionList {
                        items: expected_items,
                        ..
                    }),
                    CompletionResponse::Array(actual_items),
                ) => {
                    writeln!(
                        f,
                        "Expected `CompletionResponse::List`, got `CompletionResponse::Array`"
                    )?;
                    <Vec<CompletionItem>>::compare(
                        f,
                        Some("CompletionResponse"),
                        expected_items,
                        actual_items,
                        0,
                        None,
                    )?;
                }
            },
        }

        Ok(())
    }
}

impl Compare for CompletionResponse {
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
            (Self::Array(expected), Self::Array(actual)) => {
                writeln!(f, "{padding}{name_str}CompletionResponse::Array (")?;
                <Vec<CompletionItem>>::compare(
                    f,
                    None,
                    expected,
                    actual,
                    depth + 1,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            (
                Self::List(CompletionList {
                    is_incomplete: expected_is_incomplete,
                    items: expected_items,
                }),
                Self::List(CompletionList {
                    is_incomplete: actual_is_incomplete,
                    items: actual_items,
                }),
            ) => {
                writeln!(f, "{padding}{name_str}CompletionResponse::List (")?;
                bool::compare(
                    f,
                    Some("is_incomplete"),
                    expected_is_incomplete,
                    actual_is_incomplete,
                    depth + 1,
                    override_color,
                )?;
                <Vec<CompletionItem>>::compare(
                    f,
                    Some("items"),
                    expected_items,
                    actual_items,
                    depth + 1,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            _ => {
                cmp_fallback(f, expected, actual, depth, name, override_color)?;
            }
        }
        Ok(())
    }
}

impl Compare for CompletionItem {
    type Nested1 = ();
    type Nested2 = ();
    #[allow(clippy::too_many_lines)]
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
        writeln!(f, "{padding}{name_str}CompletionItem {{")?;
        String::compare(
            f,
            Some("label"),
            &expected.label,
            &actual.label,
            depth,
            override_color,
        )?;
        <Option<CompletionItemLabelDetails>>::compare(
            f,
            Some("label_details"),
            &expected.label_details,
            &actual.label_details,
            depth,
            override_color,
        )?;
        <Option<CompletionItemKind>>::compare(
            f,
            Some("kind"),
            &expected.kind,
            &actual.kind,
            depth,
            override_color,
        )?;
        <Option<String>>::compare(
            f,
            Some("detail"),
            &expected.detail,
            &actual.detail,
            depth,
            override_color,
        )?;
        <Option<Documentation>>::compare(
            f,
            Some("documentation"),
            &expected.documentation,
            &actual.documentation,
            depth,
            override_color,
        )?;
        <Option<bool>>::compare(
            f,
            Some("deprecated"),
            &expected.deprecated,
            &actual.deprecated,
            depth,
            override_color,
        )?;
        <Option<bool>>::compare(
            f,
            Some("preselect"),
            &expected.preselect,
            &actual.preselect,
            depth,
            override_color,
        )?;
        <Option<String>>::compare(
            f,
            Some("sort_text"),
            &expected.sort_text,
            &actual.sort_text,
            depth,
            override_color,
        )?;
        <Option<String>>::compare(
            f,
            Some("filter_text"),
            &expected.filter_text,
            &actual.filter_text,
            depth,
            override_color,
        )?;
        <Option<String>>::compare(
            f,
            Some("insert_text"),
            &expected.insert_text,
            &actual.insert_text,
            depth,
            override_color,
        )?;
        <Option<InsertTextFormat>>::compare(
            f,
            Some("insert_text_format"),
            &expected.insert_text_format,
            &actual.insert_text_format,
            depth,
            override_color,
        )?;
        <Option<InsertTextMode>>::compare(
            f,
            Some("insert_text_mode"),
            &expected.insert_text_mode,
            &actual.insert_text_mode,
            depth,
            override_color,
        )?;
        <Option<CompletionTextEdit>>::compare(
            f,
            Some("text_edit"),
            &expected.text_edit,
            &actual.text_edit,
            depth,
            override_color,
        )?;
        <Option<Vec<TextEdit>>>::compare(
            f,
            Some("additional_text_edits"),
            &expected.additional_text_edits,
            &actual.additional_text_edits,
            depth,
            override_color,
        )?;
        <Option<Command>>::compare(
            f,
            Some("command"),
            &expected.command,
            &actual.command,
            depth,
            override_color,
        )?;
        <Option<Vec<String>>>::compare(
            f,
            Some("commit_characters"),
            &expected.commit_characters,
            &actual.commit_characters,
            depth,
            override_color,
        )?;
        <Option<serde_json::Value>>::compare(
            f,
            Some("data"),
            &expected.data,
            &actual.data,
            depth,
            override_color,
        )?;
        <Option<Vec<CompletionItemTag>>>::compare(
            f,
            Some("tags"),
            &expected.tags,
            &actual.tags,
            depth,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for CompletionItemLabelDetails {
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
        writeln!(f, "{padding}{name_str}CompletionItemLabelDetails {{")?;
        <Option<String>>::compare(
            f,
            Some("detail"),
            &expected.detail,
            &actual.detail,
            depth + 1,
            override_color,
        )?;
        <Option<String>>::compare(
            f,
            Some("description"),
            &expected.description,
            &actual.description,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for CompletionItemKind {
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

impl Compare for Documentation {
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
                writeln!(f, "{padding}{name_str}Documentation::String (")?;
                String::compare(f, None, expected, actual, depth + 1, override_color)?;
                writeln!(f, "{padding})")?;
            }
            (Self::MarkupContent(expected), Self::MarkupContent(actual)) => {
                writeln!(f, "{padding}{name_str}Documentation::MarkupContent (")?;
                MarkupContent::compare(f, None, expected, actual, depth + 1, override_color)?;
                writeln!(f, "{padding})")?;
            }
            _ => {
                cmp_fallback(f, expected, actual, depth, name, override_color)?;
            }
        }

        Ok(())
    }
}

impl Compare for InsertTextFormat {
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

impl Compare for InsertTextMode {
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

impl Compare for CompletionTextEdit {
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
            (Self::Edit(expected_edit), Self::Edit(actual_edit)) => {
                writeln!(f, "{padding}{name_str}CompletionTextEdit::Edit (")?;
                TextEdit::compare(
                    f,
                    None,
                    expected_edit,
                    actual_edit,
                    depth + 1,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            (Self::InsertAndReplace(expected_text), Self::InsertAndReplace(actual_text)) => {
                writeln!(f, "{padding}{name_str}CompletionTextEdit::InsertText (")?;
                InsertReplaceEdit::compare(
                    f,
                    None,
                    expected_text,
                    actual_text,
                    depth + 1,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            _ => {
                cmp_fallback(f, expected, actual, depth, name, override_color)?;
            }
        }

        Ok(())
    }
}

impl Compare for InsertReplaceEdit {
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
        writeln!(f, "{padding}{name_str}InsertReplaceEdit {{")?;
        String::compare(
            f,
            Some("new_text"),
            &expected.new_text,
            &actual.new_text,
            depth,
            override_color,
        )?;
        Range::compare(
            f,
            Some("insert"),
            &expected.insert,
            &actual.insert,
            depth,
            override_color,
        )?;
        Range::compare(
            f,
            Some("replace"),
            &expected.replace,
            &actual.replace,
            depth,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for Command {
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
        writeln!(f, "{padding}{name_str}Command {{")?;
        String::compare(
            f,
            Some("title"),
            &expected.title,
            &actual.title,
            depth,
            override_color,
        )?;
        String::compare(
            f,
            Some("command"),
            &expected.command,
            &actual.command,
            depth,
            override_color,
        )?;
        <Option<Vec<serde_json::Value>>>::compare(
            f,
            Some("arguments"),
            &expected.arguments,
            &actual.arguments,
            depth,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for CompletionItemTag {
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
