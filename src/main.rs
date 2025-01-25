// NOTE: Sample usage of the library, to be deleted later on
use std::path::PathBuf;

use lspresso_shot::{
    lspresso_shot, test_completions, test_definition, test_diagnostics, test_hover,
    types::{
        CompletionResult, CursorPosition, DefinitionResult, DiagnosticInfo, DiagnosticResult,
        DiagnosticSeverity, HoverResult, TestCase,
    },
};

#[test]
fn rust_analyzer_hover() {
    let hover_test_case = TestCase::new(
        "src/main.rs",
        "rust-analyzer",
        r#"pub fn main() {
    println!("Hello, world!");
}"#,
    )
    .cursor_pos(Some(CursorPosition::new(2, 5)))
    .other_file(
        "Cargo.toml",
        r#"
[package]
name = "src"
version = "0.1.0"
edition = "2021"

[dependencies]

[[bin]]
name = "src"
path = "src/main.rs"
"#,
    );
    lspresso_shot!(test_hover(
        hover_test_case,
        HoverResult {
            kind: "markdown".to_string(),
            value: r#"
                \n```rust\nstd::macros\n```\n\n```rust\nmacro_rules! println
 // matched arm #1\n```\n\n---\n\nPrints to the standard output, with a newli
ne.\n\nOn all platforms, the newline is the LINE FEED character (`\\n`/`U+000
A`) alone\n(no additional CARRIAGE RETURN (`\\r`/`U+000D`)).\n\nThis macro us
es the same syntax as [`format`](https://doc.rust-lang.org/stable/alloc/macro
s/macro.format.html), but writes to the standard output instead.\nSee [`std::
fmt`] for more information.\n\nThe `println!` macro will lock the standard ou
tput on each call. If you call\n`println!` within a hot loop, this behavior m
ay be the bottleneck of the loop.\nTo avoid this, lock stdout with [`io::stdo
ut().lock`](https://doc.rust-lang.org/stable/std/io/stdio/struct.Stdout.html)
:\n\n```rust\nuse std::io::{stdout, Write};\n\nlet mut lock = stdout().lock()
;\nwriteln!(lock, "hello world").unwrap();\n```\n\nUse `println!` only for th
e primary output of your program. Use\n[`eprintln`] instead to print error an
d progress messages.\n\nSee [the formatting documentation in `std::fmt`](http
s://doc.rust-lang.org/stable/std/std/fmt/index.html)\nfor details of the macr
o argument syntax.\n\n# Panics\n\nPanics if writing to [`io::stdout`] fails.\
n\nWriting to non-blocking stdout can cause an error, which will lead\nthis m
acro to panic.\n\n# Examples\n\n```rust\nprintln!(); // prints just a newline
\nprintln!("hello there!");\nprintln!("format {} arguments", "some");\nlet lo
cal_variable = "some";\nprintln!("format {local_variable} arguments");\n```"#
                .to_string()
        }
    ))
}

pub fn main() {
    let lsp_path = PathBuf::from("/home/lillis/projects/asm-lsp/target/debug/asm-lsp");
    // Only add a source file to the case, test the default config
    let hover_test_case = TestCase::new(
        "gas.s",
        &lsp_path,
        include_str!("../../asm-lsp/samples/gas.s"),
    )
    .cursor_pos(Some(CursorPosition::new(20, 10)));
    lspresso_shot!(test_hover(
        hover_test_case,
        HoverResult {
            kind: "markdown".to_string(),
            value: "RBP [x86-64]
Base Pointer (meant for stack frames)


Type: General Purpose Register
Width: 64 bits
"
            .to_string(),
        },
    ));

    // Add a source and config file to the case case!
    let diagnostic_test_case = TestCase::new(
        "gas.s",
        &lsp_path,
        include_str!("../../asm-lsp/samples/gas.s"),
    )
    .other_file(
        ".asm-lsp.toml",
        r#"
            [default_config]
    version = "0.9.0"
    assembler = "gas"
    instruction_set = "x86-64"

    [default_config.opts]
    compiler = "zig"
    compile_flags_txt = ["cc"]
    diagnostics = true
    default_diagnostics = true"#,
    );
    lspresso_shot!(test_diagnostics(
        diagnostic_test_case,
        &DiagnosticResult {
            diagnostics: vec![DiagnosticInfo {
                start_line: 25,
                start_character: 2,
                end_line: Some(25),
                end_character: Some(2),
                message: "error: too few operands for instruction\n".to_string(),
                severity: Some(DiagnosticSeverity::Error)
            }],
        },
    ));

    let completion_test_case = TestCase::new(
        "gas.s",
        &lsp_path,
        include_str!("../../asm-lsp/samples/gas.s"),
    )
    .cursor_pos(Some(CursorPosition::new(1, 4)));
    lspresso_shot!(test_completions(
        completion_test_case,
        &CompletionResult::MoreThan(1),
    ));

    let definition_test_case = TestCase::new(
        "gas.s",
        &lsp_path,
        include_str!("../../asm-lsp/samples/gas.s"),
    )
    .cursor_pos(Some(CursorPosition::new(15, 8)));
    lspresso_shot!(test_definition(
        definition_test_case,
        &DefinitionResult {
            start_pos: CursorPosition::new(16, 0),
            end_pos: Some(CursorPosition::new(16, 5)),
            path: PathBuf::from("src/gas.s"),
        },
    ));
}
