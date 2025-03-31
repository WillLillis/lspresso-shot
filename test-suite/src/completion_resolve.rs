#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, time::Duration};

    use crate::test_helpers::{cargo_dot_toml, NON_RESPONSE_NUM};
    use lspresso_shot::{
        lspresso_shot, test_completion_resolve,
        types::{ServerStartType, TestCase, TestError, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{
        CompletionItem, CompletionItemKind, CompletionOptions, CompletionTextEdit, Documentation,
        InsertTextFormat, MarkupContent, Position, Range, ServerCapabilities, TextEdit,
    };
    use rstest::rstest;

    fn completion_resolve_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(true),
                ..Default::default()
            }),
            ..ServerCapabilities::default()
        }
    }

    fn get_dummy_completion(test_case: &TestCase) -> CompletionItem {
        let uri = test_case.get_source_file_path("").unwrap();
        CompletionItem {
            data: Some(serde_json::json!({ "uri": &uri })),
            ..Default::default()
        }
    }

    #[test]
    fn test_server_completion_resolve_simple_expect_none_got_none() {
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(NON_RESPONSE_NUM, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&completion_resolve_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");
        let completion_item = get_dummy_completion(&test_case);

        lspresso_shot!(test_completion_resolve(test_case, &completion_item, None));
    }

    #[rstest]
    fn test_server_completion_simple_expect_none_got_some(#[values(0, 1)] response_num: u32) {
        let resp = test_server::responses::get_completion_resolve_response(response_num).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&completion_resolve_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");
        let completion_item = get_dummy_completion(&test_case);

        let test_result = test_completion_resolve(test_case.clone(), &completion_item, None);
        let expected_err = TestError::ExpectedNone(test_case.test_id, format!("{resp:#?}"));
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_completion_simple_expect_some_got_some(#[values(0, 1)] response_num: u32) {
        let resp = test_server::responses::get_completion_resolve_response(response_num).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&completion_resolve_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");
        let completion_item = get_dummy_completion(&test_case);

        lspresso_shot!(test_completion_resolve(
            test_case,
            &completion_item,
            Some(&resp)
        ));
    }

    #[test]
    fn rust_analyzer_completion_resolve() {
        let source_file = TestFile::new(
            "src/main.rs",
            r#"pub fn main() {
    println!("Hello, world!");
}"#,
        );
        let test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(4).unwrap(),
                "rustAnalyzer/Indexing".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .other_file(cargo_dot_toml());

        let completion_item = CompletionItem {
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
        };

        lspresso_shot!(test_completion_resolve(
            test_case,
            &completion_item,
            Some(&completion_item)
        ));
    }
}
