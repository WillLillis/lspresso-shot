#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{
        lspresso_shot, /*test_completions,*/ test_definition, test_diagnostics, test_hover,
        types::{
            /*CompletionResult,*/ CursorPosition, DefinitionResult, DiagnosticInfo,
            DiagnosticResult, DiagnosticSeverity, HoverResult, ServerStartType, TestCase,
        },
    };

    #[test]
    fn rust_analyzer_definition() {
        let definition_test_case = TestCase::new(
            "src/main.rs",
            "rust-analyzer",
            "pub fn main() {
    let mut foo = 5;
    foo = 10;
}",
        )
        .start_type(ServerStartType::Progress(
            "rustAnalyzer/Indexing".to_string(),
        ))
        .cursor_pos(Some(CursorPosition::new(2, 5)))
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

        lspresso_shot!(test_definition(
            definition_test_case,
            &DefinitionResult {
                start_pos: CursorPosition::new(1, 8),
                end_pos: Some(CursorPosition::new(1, 15)),
                path: "src/main.rs".into(),
            },
        ));
    }

    #[test]
    fn rust_analyzer_diagnostics() {
        // Add a source and config file to the case case!
        let diagnostic_test_case = TestCase::new(
            "src/main.rs",
            "rust-analyzer",
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

        lspresso_shot!(test_diagnostics(
            diagnostic_test_case,
            &DiagnosticResult {
                diagnostics: vec![DiagnosticInfo {
                    start_line: 1,
                    start_character: 13,
                    end_line: Some(2),
                    end_character: Some(13),
                    message: "unterminated double quote string\n".to_string(),
                    severity: Some(DiagnosticSeverity::Error)
                }],
            },
        ));
    }

    #[test]
    fn rust_analyzer_hover() {
        let hover_test_case = TestCase::new(
            "src/main.rs",
            "rust-analyzer",
            r#"pub fn main() {
    println!("Hello, world!");
}"#,
        )
        .start_type(ServerStartType::Progress(
            "rustAnalyzer/Indexing".to_string(),
        ))
        .timeout(Duration::from_secs(10)) // rust-analyzer is *slow* to startup cold
        .cursor_pos(Some(CursorPosition::new(1, 5)))
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
        HoverResult {
            kind: "markdown".to_string(),
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
```
".to_string()
        }
    ));
    }

    // TODO: Need to rethink how to test completions
    //     #[test]
    //     fn rust_analyzer_completion() {
    //         let completion_test_case = TestCase::new(
    //             "src/main.rs",
    //             "rust-analyzer",
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
