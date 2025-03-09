#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, str::FromStr as _, time::Duration};

    use crate::test_helpers::{cargo_dot_toml, NON_RESPONSE_NUM};
    use lspresso_shot::{
        lspresso_shot, test_implementation,
        types::{ServerStartType, TestCase, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{
        GotoDefinitionResponse, ImplementationProviderCapability, LocationLink, Position, Range,
        ServerCapabilities, Uri,
    };
    use rstest::rstest;

    fn implementation_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            implementation_provider: Some(ImplementationProviderCapability::Simple(true)),
            ..Default::default()
        }
    }

    #[test]
    fn test_server_implementation_empty_simple() {
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file)
            .cursor_pos(Some(Position::default()));
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(NON_RESPONSE_NUM, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&implementation_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_implementation(test_case, None));
    }

    #[rstest]
    fn test_server_implementation_simple(#[values(0, 1, 2, 3, 4, 5, 6)] response_num: u32) {
        let resp = test_server::responses::get_implementation_response(response_num).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file)
            .cursor_pos(Some(Position::default()));
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&implementation_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_implementation(test_case, Some(&resp)));
    }

    #[test]
    fn rust_analyzer_implementation() {
        let source_file = TestFile::new(
            "src/main.rs",
            "struct Foo {
    x: i32,
}

trait Bar {
    fn bar(&self);
}

impl Bar for Foo {
    fn bar(&self) {}
}

pub fn main() {
    let foo = Foo { x: 42 };
    foo.bar();
}",
        );
        let test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(5).unwrap(),
                "rustAnalyzer/Indexing".to_string(),
            ))
            .cursor_pos(Some(Position::new(14, 10)))
            .timeout(Duration::from_secs(20))
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_implementation(
            test_case,
            Some(&GotoDefinitionResponse::Link(vec![LocationLink {
                target_uri: Uri::from_str("src/main.rs").unwrap(),
                origin_selection_range: Some(Range {
                    start: Position {
                        line: 14,
                        character: 8,
                    },
                    end: Position {
                        line: 14,
                        character: 11,
                    },
                }),
                target_range: Range {
                    start: Position {
                        line: 9,
                        character: 4,
                    },
                    end: Position {
                        line: 9,
                        character: 20,
                    },
                },
                target_selection_range: Range {
                    start: Position {
                        line: 9,
                        character: 7,
                    },
                    end: Position {
                        line: 9,
                        character: 10,
                    },
                },
            }]))
        ));
    }
}
