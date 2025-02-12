#[cfg(test)]
mod tests {
    use std::{str::FromStr, time::Duration};

    use lsp_types::{
        CodeDescription, Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity,
        DiagnosticTag, GotoDefinitionResponse, Hover, Location, LocationLink, MarkupContent,
        NumberOrString, Position, Range, Uri,
    };
    use serde_json::Map;

    use crate::{
        lspresso_shot, /*test_completions,*/ test_definition, test_diagnostics, test_hover,
        types::{ServerStartType, TestCase},
    };

    #[test]
    fn rust_analyzer_definition() {
        let definition_test_case = TestCase::new(
            "rust-analyzer",
            "src/main.rs",
            "pub fn main() {
    let mut foo = 5;
    foo = 10;
}",
        )
        .start_type(ServerStartType::Progress(
            "rustAnalyzer/Indexing".to_string(),
        ))
        .cursor_pos(Some(Position::new(2, 5)))
        .timeout(Duration::from_secs(10)) // rust-analyzer is *slow* to startup cold
        .other_file(
            "Cargo.toml",
            r#"
[package]
name = "test"
version = "0.1.0"
edition = "2021"

[dependencies]

[[bin]]
name = "test"
path = "src/main.rs"
"#,
        );

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

    #[test]
    fn rust_analyzer_multi_diagnostics() {
        // Add a source and config file to the case case!
        let diagnostic_test_case = TestCase::new(
            "rust-analyzer",
            "src/main.rs",
            "pub fn main() {
    let bar = 1;
}",
        )
        .start_type(ServerStartType::Progress(
            "rustAnalyzer/Indexing".to_string(),
        ))
        .timeout(Duration::from_secs(5)) // rust-analyzer is *slow* to startup cold
        .other_file(
            "Cargo.toml",
            r#"
[package]
name = "test"
version = "0.1.0"
edition = "2021"

[dependencies]

[[bin]]
name = "test"
path = "src/main.rs"
"#,
        );

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

    #[test]
    fn rust_analyzer_diagnostics() {
        // Add a source and config file to the case case!
        let diagnostic_test_case = TestCase::new(
            "rust-analyzer",
            "src/main.rs",
            r#"pub fn main() {
    println!("Hello, world!
}"#,
        )
        .start_type(ServerStartType::Progress(
            "rustAnalyzer/Indexing".to_string(),
        ))
        .timeout(Duration::from_secs(5)) // rust-analyzer is *slow* to startup cold
        .other_file(
            "Cargo.toml",
            r#"
[package]
name = "test"
version = "0.1.0"
edition = "2021"

[dependencies]

[[bin]]
name = "test"
path = "src/main.rs"
"#,
        );

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
        let hover_test_case = TestCase::new(
            "rust-analyzer",
            "src/main.rs",
            r#"pub fn main() {
    println!("Hello, world!");
}"#,
        )
        .start_type(ServerStartType::Progress(
            "rustAnalyzer/Indexing".to_string(),
        ))
        .timeout(Duration::from_secs(10)) // rust-analyzer is *slow* to startup cold
        .cursor_pos(Some(Position::new(1, 5)))
        .other_file(
            "Cargo.toml",
            r#"
[package]
name = "test"
version = "0.1.0"
edition = "2021"

[dependencies]

[[bin]]
name = "test"
path = "src/main.rs"
"#,
        );

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

    // TODO: Need to rethink how to test completions
    //     #[test]
    //     fn rust_analyzer_completion() {
    //         let completion_test_case = TestCase::new(
    //             "rust-analyzer",
    //             "src/main.rs",
    //             r#"pub fn main() {
    //     prin
    // }"#,
    //         )
    //         .start_type(ServerStartType::Progress(
    //             "rustAnalyzer/Indexing".to_string(),
    //         ))
    //         .timeout(Duration::from_secs(10)) // rust-analyzer is *slow* to startup cold
    //         .cursor_pos(Some(CursorPosition::new(1, 8)))
    //         .other_file(
    //             "Cargo.toml",
    //             r#"
    // [package]
    // name = "test"
    // version = "0.1.0"
    // edition = "2021"
    //
    // [dependencies]
    //
    // [[bin]]
    // name = "test"
    // path = "src/main.rs"
    // "#,
    //         );
    //         lspresso_shot!(test_completions(
    //             completion_test_case,
    //             &CompletionResult::MoreThan(1),
    //         ));
    //     }
}
