use lsp_types::{CompletionItem, CompletionList, CompletionResponse};
use thiserror::Error;

use super::{paint, write_fields_comparison, CleanResponse, Empty, RED};

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
                    };
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
                    };
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
                    write_fields_comparison(
                        f,
                        "CompletionResponse",
                        expected_results,
                        &self.actual,
                        0,
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
                    write_fields_comparison(
                        f,
                        "CompletionResponse",
                        expected_items,
                        actual_items,
                        0,
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
                    write_fields_comparison(
                        f,
                        "CompletionResponse",
                        expected_items,
                        actual_items,
                        0,
                    )?;
                }
            },
        };

        Ok(())
    }
}
