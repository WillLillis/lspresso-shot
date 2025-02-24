#[cfg(test)]
mod tests {
    use std::{num::NonZeroU32, path::PathBuf, str::FromStr, time::Duration};

    use lsp_types::{
        CodeDescription, CompletionItem, CompletionItemKind, CompletionTextEdit, Diagnostic,
        DiagnosticRelatedInformation, DiagnosticSeverity, DiagnosticTag, DocumentChanges,
        Documentation, GotoDefinitionResponse, Hover, InsertTextFormat, Location, LocationLink,
        MarkupContent, NumberOrString, OneOf, OptionalVersionedTextDocumentIdentifier, Position,
        Range, TextDocumentEdit, TextEdit, Uri, WorkspaceEdit,
    };
    use serde_json::Map;

    use crate::{
        lspresso_shot, test_completion, test_definition, test_diagnostics, test_formatting,
        test_hover, test_references, test_rename,
        types::{CompletionResult, FormattingResult, ServerStartType, TestCase, TestFile},
    };

    // NOTE: Timouts are set to ridiculous values for these to avoid issues with
    // slow CI runners. For local testing, 5-15 seconds should be sufficient

    fn get_dummy_server_path() -> PathBuf {
        let mut proj_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        proj_dir.push("target");
        proj_dir.push("debug");
        proj_dir.push("test-server");

        proj_dir
    }

    fn cargo_dot_toml() -> TestFile {
        TestFile::new(
            "Cargo.toml",
            r#"
[package]
name = "test"
version = "0.1.0"
edition = "2021"

[dependencies]

[[bin]]
name = "test"
path = "src/main.rs""#,
        )
    }

    #[test]
    fn dummy_references() {
        let mut response_num = 1;
        while let Some(refs) = test_server::responses::get_references_response(response_num) {
            let source_file = TestFile::new(test_server::responses::get_source_path(), "");
            let reference_test_case = TestCase::new(get_dummy_server_path(), source_file)
                .cursor_pos(Some(Position::new(response_num, 0)))
                .timeout(Duration::from_secs(1))
                .cleanup(false);

            lspresso_shot!(test_references(reference_test_case, true, &refs,));
            response_num += 1;
        }
    }

    #[test]
    fn rust_analyzer_references() {
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
    let foo = 5;
}",
        );
        let reference_test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(5).unwrap(),
                "rustAnalyzer/Indexing".to_string(),
            ))
            .cursor_pos(Some(Position::new(1, 9)))
            .timeout(Duration::from_secs(20))
            .cleanup(false)
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_references(
            reference_test_case,
            true,
            &vec![Location {
                uri: Uri::from_str("src/main.rs").unwrap(),
                range: Range {
                    start: Position::new(1, 8),
                    end: Position::new(1, 11)
                },
            }]
        ));
    }

    #[test]
    fn rust_analyzer_formatting_state() {
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
let foo = 5;
}",
        );
        let formatting_test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(1).unwrap(),
                "rustAnalyzer/Indexing".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .cleanup(false)
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_formatting(
            formatting_test_case,
            None,
            &FormattingResult::EndState(
                "pub fn main() {
    let foo = 5;
}
"
                .to_string()
            )
        ));
    }

    #[test]
    fn rust_analyzer_formatting_response() {
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
let foo = 5;
}",
        );
        let formatting_test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(1).unwrap(),
                "rustAnalyzer/Indexing".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .cleanup(false)
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_formatting(
            formatting_test_case,
            None,
            &FormattingResult::Response(vec![
                TextEdit {
                    new_text: "    ".to_string(),
                    range: Range {
                        start: Position::new(1, 0),
                        end: Position::new(1, 0),
                    },
                },
                TextEdit {
                    new_text: "\n".to_string(),
                    range: Range {
                        start: Position::new(2, 1),
                        end: Position::new(2, 1),
                    }
                }
            ]),
        ));
    }

    #[test]
    fn rust_analyzer_rename() {
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
    let foo = 5;
}",
        );
        let rename_test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(5).unwrap(),
                "rustAnalyzer/Indexing".to_string(),
            ))
            .cursor_pos(Some(Position::new(1, 9)))
            .timeout(Duration::from_secs(20))
            .cleanup(false)
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_rename(
            rename_test_case,
            "bar",
            &WorkspaceEdit {
                changes: None,
                document_changes: Some(DocumentChanges::Edits(vec![TextDocumentEdit {
                    text_document: OptionalVersionedTextDocumentIdentifier {
                        uri: Uri::from_str("src/main.rs").unwrap(),
                        version: Some(0)
                    },
                    edits: vec![OneOf::Left(TextEdit {
                        range: Range {
                            start: Position::new(1, 8),
                            end: Position::new(1, 11)
                        },
                        new_text: "bar".to_string()
                    })]
                }])),
                change_annotations: None
            }
        ));
    }

    #[test]
    fn rust_analyzer_definition() {
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
    let mut foo = 5;
    foo = 10;
}",
        );
        let definition_test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(5).unwrap(),
                "rustAnalyzer/Indexing".to_string(),
            ))
            .cursor_pos(Some(Position::new(2, 5)))
            .timeout(Duration::from_secs(20))
            .cleanup(false)
            .other_file(cargo_dot_toml());

        // TODO: Add test for multiple definitions returned
        lspresso_shot!(test_definition(
            definition_test_case,
            &GotoDefinitionResponse::Link(vec![LocationLink {
                target_uri: Uri::from_str("src/main.rs").unwrap(),
                origin_selection_range: Some(Range {
                    start: Position {
                        line: 2,
                        character: 4,
                    },
                    end: Position {
                        line: 2,
                        character: 7,
                    },
                }),
                target_range: Range {
                    start: Position {
                        line: 1,
                        character: 8,
                    },
                    end: Position {
                        line: 1,
                        character: 15,
                    },
                },
                target_selection_range: Range {
                    start: Position {
                        line: 1,
                        character: 12,
                    },
                    end: Position {
                        line: 1,
                        character: 15,
                    },
                },
            }])
        ));
    }

    // NOTE:: Specifying the start type is ignored for diagnostics tests
    #[test]
    fn rust_analyzer_multi_diagnostics() {
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
    let bar = 1;
}",
        );
        let diagnostic_test_case = TestCase::new("rust-analyzer", source_file)
            .timeout(Duration::from_secs(20))
            .cleanup(false)
            .other_file(cargo_dot_toml());

        let mut data_map = Map::new();
        data_map.insert(
            "rendered".to_string(),
            serde_json::Value::String("warning: unused variable: `bar`\n --> src/main.rs:2:9\n  |\n2 |     let bar = 1;\n  |         ^^^ help: if this is intentional, prefix it with an underscore: `_bar`\n  |\n  = note: `#[warn(unused_variables)]` on by default\n\n".to_string()),
        );
        let uri = Uri::from_str("src/main.rs").unwrap();
        let range = Range {
            start: Position {
                line: 1,
                character: 8,
            },
            end: Position {
                line: 1,
                character: 11,
            },
        };
        lspresso_shot!(test_diagnostics(
            diagnostic_test_case,
            &vec![
                Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::WARNING),
                    code: Some(NumberOrString::String("unused_variables".to_string())),
                    code_description: None,
                    source: Some("rustc".to_string()),
                    message: "unused variable: `bar`\n`#[warn(unused_variables)]` on by default"
                        .to_string(),
                    related_information: Some(vec![DiagnosticRelatedInformation {
                        location: Location {
                            uri: uri.clone(),
                            range,
                        },
                        message: "if this is intentional, prefix it with an underscore: `_bar`"
                            .to_string(),
                    }]),
                    tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                    data: Some(serde_json::Value::Object(data_map)),
                },
                Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::HINT),
                    code: Some(NumberOrString::String("unused_variables".to_string())),
                    code_description: None,
                    source: Some("rustc".to_string()),
                    message: "if this is intentional, prefix it with an underscore: `_bar`"
                        .to_string(),
                    related_information: Some(vec![DiagnosticRelatedInformation {
                        location: Location { uri, range },
                        message: "original diagnostic".to_string(),
                    }]),
                    tags: None,
                    data: None,
                }
            ],
        ));
    }

    // NOTE:: Specifying the start type is ignored for diagnostics tests
    #[test]
    fn rust_analyzer_diagnostics() {
        let source_file = TestFile::new(
            "src/main.rs",
            r#"pub fn main() {
    println!("Hello, world!
}"#,
        );
        let diagnostic_test_case = TestCase::new("rust-analyzer", source_file)
            .timeout(Duration::from_secs(20))
            .cleanup(false)
            .other_file(cargo_dot_toml());

        let mut data_map = Map::new();
        _ = data_map.insert(
            "rendered".to_string(),
            serde_json::Value::String("error[E0765]: unterminated double quote string\n --> src/main.rs:2:14\n  |\n2 |       println!(\"Hello, world!\n  |  ______________^\n3 | | }\n  | |_^\n\n".to_string()),
        );
        lspresso_shot!(test_diagnostics(
            diagnostic_test_case,
            &vec![Diagnostic {
                range: Range {
                    start: Position {
                        line: 1,
                        character: 13,
                    },
                    end: Position {
                        line: 2,
                        character: 1,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(NumberOrString::String("E0765".to_string())),
                code_description: Some(CodeDescription {
                    href: lsp_types::Uri::from_str(
                        "https://doc.rust-lang.org/error-index.html#E0765"
                    )
                    .unwrap()
                }),
                source: Some("rustc".to_string()),
                message: "unterminated double quote string".to_string(),
                related_information: None,
                tags: None,
                data: Some(serde_json::Value::Object(data_map)),
            }],
        ));
    }

    #[test]
    fn rust_analyzer_hover() {
        let source_file = TestFile::new(
            "src/main.rs",
            r#"pub fn main() {
    println!("Hello, world!");
}"#,
        );
        let hover_test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(1).unwrap(),
                "rustAnalyzer/Indexing".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .cleanup(false)
            .cursor_pos(Some(Position::new(1, 5)))
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_hover(
        hover_test_case,
        Hover {
            range: Some(Range {
                start: lsp_types::Position {
                    line: 1,
                    character: 4,
                },
                end: lsp_types::Position {
                    line: 1,
                    character: 11,
                },
            }),
            contents: lsp_types::HoverContents::Markup(MarkupContent {
                kind: lsp_types::MarkupKind::Markdown,
                value:
                "
```rust
std::macros
```

```rust
macro_rules! println // matched arm #1
```

---

Prints to the standard output, with a newline.

On all platforms, the newline is the LINE FEED character (`\\n`/`U+000A`) alone
(no additional CARRIAGE RETURN (`\\r`/`U+000D`)).

This macro uses the same syntax as [`format`](https://doc.rust-lang.org/stable/alloc/macros/macro.format.html), but writes to the standard output instead.
See [`std::fmt`] for more information.

The `println!` macro will lock the standard output on each call. If you call
`println!` within a hot loop, this behavior may be the bottleneck of the loop.
To avoid this, lock stdout with [`io::stdout().lock`](https://doc.rust-lang.org/stable/std/io/stdio/struct.Stdout.html):

```rust
use std::io::{stdout, Write};

let mut lock = stdout().lock();
writeln!(lock, \"hello world\").unwrap();
```

Use `println!` only for the primary output of your program. Use
[`eprintln`] instead to print error and progress messages.

See [the formatting documentation in `std::fmt`](https://doc.rust-lang.org/stable/std/std/fmt/index.html)
for details of the macro argument syntax.

# Panics

Panics if writing to [`io::stdout`] fails.

Writing to non-blocking stdout can cause an error, which will lead
this macro to panic.

# Examples

```rust
println!(); // prints just a newline
println!(\"hello there!\");
println!(\"format {} arguments\", \"some\");
let local_variable = \"some\";
println!(\"format {local_variable} arguments\");
```".to_string()
            })
        }
    ));
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
            deprecated: Some(false),
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
            additional_text_edits: Some(vec![]),
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
        let completion_test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(5).unwrap(),
                "rustAnalyzer/Indexing".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .cleanup(false)
            .cursor_pos(Some(Position::new(1, 9)))
            .other_file(cargo_dot_toml());
        lspresso_shot!(test_completion(completion_test_case, &expected_comps));
    }
}
