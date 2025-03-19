#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, str::FromStr as _, time::Duration};

    use crate::test_helpers::{cargo_dot_toml, NON_RESPONSE_NUM};
    use lspresso_shot::{
        lspresso_shot, test_type_definition,
        types::{ServerStartType, TestCase, TestError, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{
        GotoDefinitionResponse, LocationLink, Position, Range, ServerCapabilities,
        TypeDefinitionProviderCapability, Uri,
    };
    use rstest::rstest;

    fn type_definition_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(true)),
            ..Default::default()
        }
    }

    #[test]
    fn test_server_type_definition_simple_expect_none_got_none() {
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(NON_RESPONSE_NUM, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&type_definition_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_type_definition(test_case, &Position::default(), None));
    }

    #[rstest]
    fn test_server_type_definition_simple_expect_none_got_some(
        #[values(0, 1, 2, 3, 4, 5, 6)] response_num: u32,
    ) {
        let resp = test_server::responses::get_type_definition_response(response_num).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&type_definition_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        let test_result = test_type_definition(test_case.clone(), &Position::default(), None);
        let mut expected_err =
            TestError::ExpectedNone(test_case.test_id.clone(), format!("{resp:#?}"));
        if response_num == 3 {
            // HACK: Because of the deserialization issues with empty vector results,
            // this error rendered incorrectly as `Link` rather than `Array`
            assert_eq!(
                expected_err,
                TestError::ExpectedNone(test_case.test_id.clone(), "Link(\n    [],\n)".to_string())
            );
            expected_err =
                TestError::ExpectedNone(test_case.test_id, "Array(\n    [],\n)".to_string());
        }
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_type_definition_simple_expect_some_got_some(
        #[values(0, 1, 2, 3, 4, 5, 6)] response_num: u32,
    ) {
        let resp = test_server::responses::get_type_definition_response(response_num).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&type_definition_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_type_definition(
            test_case,
            &Position::default(),
            Some(&resp)
        ));
    }

    #[test]
    fn rust_analyzer_type_definition() {
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
                "rustAnalyzer/Indexing".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_type_definition(
            test_case,
            &Position::new(5, 9),
            Some(&GotoDefinitionResponse::Link(vec![LocationLink {
                target_uri: Uri::from_str("src/main.rs").unwrap(),
                origin_selection_range: Some(Range {
                    start: Position {
                        line: 5,
                        character: 8,
                    },
                    end: Position {
                        line: 5,
                        character: 11,
                    },
                }),
                target_range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 2,
                        character: 1,
                    },
                },
                target_selection_range: Range {
                    start: Position {
                        line: 0,
                        character: 7,
                    },
                    end: Position {
                        line: 0,
                        character: 10,
                    },
                },
            }]))
        ));
    }
}
