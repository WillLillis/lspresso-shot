#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, str::FromStr as _, time::Duration};

    use crate::test_helpers::{cargo_dot_toml, NON_RESPONSE_NUM};
    use lspresso_shot::{
        lspresso_shot, test_declaration,
        types::{ServerStartType, TestCase, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{
        DeclarationCapability, GotoDefinitionResponse, LocationLink, Position, Range,
        ServerCapabilities, Uri,
    };
    use rstest::rstest;

    fn declaration_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            declaration_provider: Some(DeclarationCapability::Simple(true)),
            ..Default::default()
        }
    }

    #[test]
    fn test_server_declaration_empty_simple() {
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file)
            .cursor_pos(Some(Position::default()));
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(NON_RESPONSE_NUM, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&declaration_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_declaration(test_case, None));
    }

    #[rstest]
    fn test_server_declaration_simple(#[values(0, 1, 2, 3, 4, 5, 6)] response_num: u32) {
        let resp = test_server::responses::get_declaration_response(response_num).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file)
            .cursor_pos(Some(Position::default()));
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&declaration_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_declaration(test_case, Some(&resp)));
    }

    #[test]
    fn rust_analyzer_declaration() {
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
    let mut foo = 5;
    foo = 10;
}",
        );
        let test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(5).unwrap(),
                "rustAnalyzer/Indexing".to_string(),
            ))
            .cursor_pos(Some(Position::new(2, 5)))
            .timeout(Duration::from_secs(20))
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_declaration(
            test_case,
            Some(&GotoDefinitionResponse::Link(vec![LocationLink {
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
            }]))
        ));
    }
}
