// NOTE: Sample usage of the library, to be deleted later on
use std::path::PathBuf;

use lspresso_shot::{
    lspresso_shot, test_hover,
    types::{CursorPosition, HoverResult, TestCase},
};

pub fn main() -> anyhow::Result<()> {
    let test_case = TestCase::new(
        "gas.s",
        include_str!("../../asm-lsp/samples/gas.s"),
        CursorPosition::new(20, 10),
    );
    lspresso_shot!(test_hover(
        &test_case,
        HoverResult {
            kind: "markdown".to_string(),
            value: "RBP [x86-64]
Base Pointer (meant for stack frames)


Type: General Purpose Register
Width: 64 bits
"
            .to_string(),
        },
        &PathBuf::from("/home/lillis/projects/asm-lsp/target/debug/asm-lsp"),
    ));

    Ok(())
}
