#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, str::FromStr as _, time::Duration};

    use crate::test_helpers::cargo_dot_toml;
    use lspresso_shot::{
        lspresso_shot, test_rename,
        types::{ServerStartType, TestCase, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{
        DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier, Position, Range,
        ServerCapabilities, TextDocumentEdit, TextEdit, Uri, WorkspaceEdit,
    };
    use rstest::rstest;

    fn rename_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            rename_provider: Some(OneOf::Left(true)),
            ..ServerCapabilities::default()
        }
    }

    #[rstest]
    fn test_server_rename_simple(#[values(1, 2, 3)] response_num: u32) {
        let edits = test_server::responses::get_rename_response(response_num).unwrap();
        let source_file = TestFile::new(test_server::get_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file)
            .cursor_pos(Some(Position::new(0, 0)))
            .cleanup(false);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&rename_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_rename(test_case, "", &edits));
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
}
