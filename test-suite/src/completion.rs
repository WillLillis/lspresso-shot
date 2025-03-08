#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, time::Duration};

    use crate::test_helpers::{cargo_dot_toml, NON_RESPONSE_NUM};
    use lspresso_shot::{
        lspresso_shot, test_completion,
        types::{CompletionResult, ServerStartType, TestCase, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{
        CompletionItem, CompletionItemKind, CompletionList, CompletionOptions,
        CompletionOptionsCompletionItem, CompletionResponse, CompletionTextEdit, Documentation,
        InsertTextFormat, MarkupContent, Position, Range, ServerCapabilities, TextEdit,
        WorkDoneProgressOptions,
    };
    use rstest::rstest;

    fn completion_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            completion_provider: Some(CompletionOptions {
                completion_item: Some(CompletionOptionsCompletionItem {
                    label_details_support: Some(true),
                }),
                trigger_characters: None,
                all_commit_characters: None,
                resolve_provider: None,
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: None,
                },
            }),
            ..Default::default()
        }
    }

    #[test]
    fn test_server_completion_exact_empty_simple() {
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file)
            .cursor_pos(Some(Position::default()));
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(NON_RESPONSE_NUM, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&completion_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_completion(test_case, None));
    }

    #[rstest]
    fn test_server_completion_exact_simple(#[values(0, 1, 2, 3, 4, 5)] response_num: u32) {
        let resp = test_server::responses::get_completion_response(response_num).unwrap();
        let comp_result = CompletionResult::Exact(resp);
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file)
            .cursor_pos(Some(Position::default()));
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&completion_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_completion(test_case, Some(&comp_result)));
    }

    #[rstest]
    fn test_server_completion_contains_simple(#[values(0, 1, 2, 3, 4, 5)] response_num: u32) {
        let resp = test_server::responses::get_completion_response(response_num).unwrap();
        let comp_result = match resp {
            CompletionResponse::Array(items)
            | CompletionResponse::List(CompletionList { items, .. }) => {
                CompletionResult::Contains(items)
            }
        };
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file)
            .cursor_pos(Some(Position::default()));
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&completion_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_completion(test_case, Some(&comp_result)));
    }

    // TODO: The end user experience for debugging completions test with CompletionResult::Contains
    // is pretty awful. If we're not going to check by struct equality, there needs
    // to be some helpers for cases where you have the "right" expected completion
    // item, but a few fields are off. Maybe write a function to sort the provided
    // results by similarity to the first unnaccounted for expected item. Then we
    // can use the json diff printing logic to help highlight differences
    #[allow(clippy::too_many_lines)]
    #[test]
    fn rust_analyzer_completion() {
        let expected_comps = CompletionResult::Contains(vec![CompletionItem {
            label: "println!(…)".to_string(),
            label_details: None,
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("macro_rules! println".to_string()),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: lsp_types::MarkupKind::Markdown,
                value: r#"Prints to the standard output, with a newline.

On all platforms, the newline is the LINE FEED character (`\n`/`U+000A`) alone
(no additional CARRIAGE RETURN (`\r`/`U+000D`)).

This macro uses the same syntax as [`format!`], but writes to the standard output instead.
See [`std::fmt`] for more information.

The `println!` macro will lock the standard output on each call. If you call
`println!` within a hot loop, this behavior may be the bottleneck of the loop.
To avoid this, lock stdout with [`io::stdout().lock()`][lock]:
```rust
use std::io::{stdout, Write};

let mut lock = stdout().lock();
writeln!(lock, "hello world").unwrap();
```

Use `println!` only for the primary output of your program. Use
[`eprintln!`] instead to print error and progress messages.

See [the formatting documentation in `std::fmt`](../std/fmt/index.html)
for details of the macro argument syntax.

[`std::fmt`]: crate::fmt
[`eprintln!`]: crate::eprintln
[lock]: crate::io::Stdout

# Panics

Panics if writing to [`io::stdout`] fails.

Writing to non-blocking stdout can cause an error, which will lead
this macro to panic.

[`io::stdout`]: crate::io::stdout

# Examples

```rust
println!(); // prints just a newline
println!("hello there!");
println!("format {} arguments", "some");
let local_variable = "some";
println!("format {local_variable} arguments");
```"#
                    .to_string(),
            })),
            deprecated: None,
            preselect: Some(true),
            sort_text: Some("7fffffff".to_string()),
            filter_text: Some("println!".to_string()),
            insert_text: None,
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            insert_text_mode: None,
            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                range: Range {
                    start: Position {
                        line: 2,
                        character: 0,
                    },
                    end: Position {
                        line: 2,
                        character: 0,
                    },
                },
                new_text: "println!($0)".to_string(),
            })),
            additional_text_edits: None,
            command: None,
            commit_characters: None,
            data: None,
            tags: None,
        }]);
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
    prin
}",
        );
        let test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(5).unwrap(),
                "rustAnalyzer/Indexing".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .cursor_pos(Some(Position::new(1, 9)))
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_completion(test_case, Some(&expected_comps)));
    }
}
