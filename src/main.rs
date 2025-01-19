// NOTE: Sample usage of the library, to be deleted later on
use std::path::PathBuf;

use lspresso_shot::{
    lspresso_shot, test_diagnostics, test_hover,
    types::{
        CursorPosition, DiagnosticInfo, DiagnosticResult, DiagnosticSeverity, HoverResult, TestCase,
    },
};

pub fn main() {
    let lsp_path = PathBuf::from("/home/lillis/projects/asm-lsp/target/debug/asm-lsp");
    // Only add a source file to the case, test the default config
    let hover_test_case = TestCase::new(
        "gas.s",
        include_str!("../../asm-lsp/samples/gas.s"),
        CursorPosition::new(20, 10),
    );
    lspresso_shot!(test_hover(
        &hover_test_case,
        HoverResult {
            kind: "markdown".to_string(),
            value: "RBP [x86-64]
Base Pointer (meant for stack frames)


Type: General Purpose Register
Width: 64 bits
"
            .to_string(),
        },
        &lsp_path
    ));

    // Add a source and config file to the case case!
    let diagnostic_test_case = TestCase::new(
        "gas.s",
        include_str!("../../asm-lsp/samples/gas.s"),
        CursorPosition::new(20, 10),
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
        &diagnostic_test_case,
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
        &lsp_path
    ));
}
