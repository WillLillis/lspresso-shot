#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, str::FromStr as _, time::Duration};

    use crate::test_helpers::{cargo_dot_toml, NON_RESPONSE_NUM};
    use lspresso_shot::{
        lspresso_shot, test_selection_range,
        types::{ServerStartType, TestCase, TestError, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{
        Position, Range, SelectionRange, SelectionRangeProviderCapability, ServerCapabilities, Uri,
    };
    use rstest::rstest;

    fn selection_range_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
            ..Default::default()
        }
    }

    #[test]
    fn test_server_simple_expect_none_got_none() {
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(NON_RESPONSE_NUM, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&selection_range_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_selection_range(test_case, &Vec::new(), None, None));
    }

    #[rstest]
    fn test_server_simple_expect_none_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp =
            test_server::responses::get_selection_range_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&selection_range_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");
        let positions = vec![Position::default()];

        let test_result = test_selection_range(test_case.clone(), &positions, None, None);
        let expected_err = TestError::ExpectedNone(test_case.test_id, format!("{resp:#?}"));
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_simple_expect_some_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp =
            test_server::responses::get_selection_range_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&selection_range_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");
        let positions = vec![Position::default(); 3];

        lspresso_shot!(test_selection_range(
            test_case,
            &positions,
            None,
            Some(&resp)
        ));
    }

    #[test]
    fn rust_analyzer() {
        let source_file = TestFile::new(
            "src/main.rs",
            "struct Foo {
        x: i32,
    }

    pub fn main() {
        let foo = Foo { x: 5 };
    }",
        );
        let test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(5).unwrap(),
                "rustAnalyzer/cachePriming".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .other_file(cargo_dot_toml());
        let positions = vec![Position::new(0, 8)];

        lspresso_shot!(test_selection_range(
            test_case,
            &positions,
            None,
            Some(&vec![SelectionRange {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 8,
                    },
                    end: Position {
                        line: 0,
                        character: 8,
                    },
                },
                parent: Some(Box::new(SelectionRange {
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 7,
                        },
                        end: Position {
                            line: 0,
                            character: 10,
                        },
                    },
                    parent: Some(Box::new(SelectionRange {
                        range: Range {
                            start: Position {
                                line: 0,
                                character: 0,
                            },
                            end: Position {
                                line: 2,
                                character: 5,
                            },
                        },
                        parent: Some(Box::new(SelectionRange {
                            range: Range {
                                start: Position {
                                    line: 0,
                                    character: 0,
                                },
                                end: Position {
                                    line: 6,
                                    character: 5,
                                },
                            },
                            parent: None,
                        }),),
                    }),),
                }),),
            },])
        ));
    }
}
