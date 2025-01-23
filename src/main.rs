// NOTE: Sample usage of the library, to be deleted later on
use std::path::PathBuf;

use lspresso_shot::{
    lspresso_shot, test_completions, test_definition, test_diagnostics, test_hover,
    types::{
        CompletionResult, CursorPosition, DefinitionResult, DiagnosticInfo, DiagnosticResult,
        DiagnosticSeverity, HoverResult, TestCase,
    },
};

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
