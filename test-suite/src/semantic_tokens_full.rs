#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, str::FromStr as _, time::Duration};

    use crate::test_helpers::{NON_RESPONSE_NUM, cargo_dot_toml};
    use lspresso_shot::{
        lspresso_shot, test_semantic_tokens_full,
        types::{
            ServerStartType, TestCase, TestError, TestFile,
            semantic_tokens::SemanticTokensFullMismatchError,
        },
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{
        SemanticToken, SemanticTokens, SemanticTokensFullOptions, SemanticTokensLegend,
        SemanticTokensOptions, SemanticTokensResult, SemanticTokensServerCapabilities,
        ServerCapabilities, Uri, WorkDoneProgressOptions,
    };
    use rstest::rstest;

    fn semantic_tokens_full_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            semantic_tokens_provider: Some(
                SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: None,
                    },
                    legend: SemanticTokensLegend::default(),
                    range: Some(false),
                    full: Some(SemanticTokensFullOptions::Bool(true)),
                }),
            ),
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
        send_capabiltiies(&semantic_tokens_full_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_semantic_tokens_full(test_case, None, None));
    }

    #[rstest]
    fn test_server_simple_expect_none_got_some(
        #[values(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)] response_num: u32,
    ) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp =
            test_server::responses::get_semantic_tokens_full_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&semantic_tokens_full_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");
        let test_result = test_semantic_tokens_full(test_case.clone(), None, None);
        let mut expected_err =
            TestError::ExpectedNone(test_case.test_id.clone(), format!("{resp:#?}"));
        match response_num {
            // TODO: See if this can be fixed up too!
            // HACK: Because of the serialization issues with `SemanticTokensResult`, we have
            // to work around
            8 => {
                assert_eq!(
                    expected_err,
                    TestError::ExpectedNone(
                        test_case.test_id.clone(),
                        "Partial(\n    SemanticTokensPartialResult {\n        data: [],\n    },\n)"
                            .to_string()
                    )
                );
                expected_err = TestError::ExpectedNone(test_case.test_id, "Tokens(\n    SemanticTokens {\n        result_id: None,\n        data: [],\n    },\n)".to_string());
            }
            9 => {
                assert_eq!(
                    expected_err,
                    TestError::ExpectedNone(test_case.test_id.clone(), "Partial(\n    SemanticTokensPartialResult {\n        data: [\n            SemanticToken {\n                delta_line: 1,\n                delta_start: 2,\n                length: 3,\n                token_type: 4,\n                token_modifiers_bitset: 5,\n            },\n        ],\n    },\n)".to_string())
                );
                expected_err = TestError::ExpectedNone(test_case.test_id, "Tokens(\n    SemanticTokens {\n        result_id: None,\n        data: [\n            SemanticToken {\n                delta_line: 1,\n                delta_start: 2,\n                length: 3,\n                token_type: 4,\n                token_modifiers_bitset: 5,\n            },\n        ],\n    },\n)".to_string());
            }
            10 => {
                assert_eq!(
                    expected_err,
                    TestError::ExpectedNone(test_case.test_id.clone(), "Partial(\n    SemanticTokensPartialResult {\n        data: [\n            SemanticToken {\n                delta_line: 5,\n                delta_start: 7,\n                length: 8,\n                token_type: 9,\n                token_modifiers_bitset: 10,\n            },\n        ],\n    },\n)".to_string())
                );
                expected_err = TestError::ExpectedNone(test_case.test_id, "Tokens(\n    SemanticTokens {\n        result_id: None,\n        data: [\n            SemanticToken {\n                delta_line: 5,\n                delta_start: 7,\n                length: 8,\n                token_type: 9,\n                token_modifiers_bitset: 10,\n            },\n        ],\n    },\n)".to_string());
            }
            11 => {
                assert_eq!(
                    expected_err,
                    TestError::ExpectedNone(test_case.test_id.clone(), "Partial(\n    SemanticTokensPartialResult {\n        data: [\n            SemanticToken {\n                delta_line: 1,\n                delta_start: 2,\n                length: 3,\n                token_type: 4,\n                token_modifiers_bitset: 5,\n            },\n            SemanticToken {\n                delta_line: 5,\n                delta_start: 7,\n                length: 8,\n                token_type: 9,\n                token_modifiers_bitset: 10,\n            },\n        ],\n    },\n)".to_string())
                );
                expected_err = TestError::ExpectedNone(test_case.test_id, "Tokens(\n    SemanticTokens {\n        result_id: None,\n        data: [\n            SemanticToken {\n                delta_line: 1,\n                delta_start: 2,\n                length: 3,\n                token_type: 4,\n                token_modifiers_bitset: 5,\n            },\n            SemanticToken {\n                delta_line: 5,\n                delta_start: 7,\n                length: 8,\n                token_type: 9,\n                token_modifiers_bitset: 10,\n            },\n        ],\n    },\n)".to_string());
            }
            _ => {}
        }
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_simple_expect_some_got_some(
        #[values(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)] response_num: u32,
    ) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp =
            test_server::responses::get_semantic_tokens_full_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&semantic_tokens_full_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_semantic_tokens_full(test_case, None, Some(&resp)));
    }

    #[test]
    fn rust_analyzer() {
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
    let foo = 5;
}",
        );
        let test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(4).unwrap(),
                "rustAnalyzer/cachePriming".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .other_file(cargo_dot_toml());

        // HACK: rust-analyzer behaves non-deterministically here w.r.t. `result_id`,
        // check for equality with the underlying expected data
        let expected_tokens = vec![
            SemanticToken {
                delta_line: 0,
                delta_start: 7,
                length: 4,
                token_type: 4,
                token_modifiers_bitset: 262_148,
            },
            SemanticToken {
                delta_line: 1,
                delta_start: 8,
                length: 3,
                token_type: 17,
                token_modifiers_bitset: 4,
            },
        ];
        if let Err(TestError::SematicTokensFullMismatch(SemanticTokensFullMismatchError {
            actual: SemanticTokensResult::Tokens(SemanticTokens { data, .. }),
            ..
        })) = test_semantic_tokens_full(
            test_case.clone(),
            None,
            Some(&SemanticTokensResult::Tokens(SemanticTokens {
                result_id: Some("4".to_string()),
                data: expected_tokens.clone(),
            })),
        ) {
            if data != expected_tokens {
                lspresso_shot!(test_semantic_tokens_full(
                    test_case,
                    None,
                    Some(&SemanticTokensResult::Tokens(SemanticTokens {
                        result_id: Some("4".to_string()),
                        data: expected_tokens
                    }))
                ));
            }
        }
    }
}
