#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, str::FromStr as _, time::Duration};

    use crate::test_helpers::cargo_dot_toml;
    use lspresso_shot::{
        lspresso_shot, test_semantic_tokens_full_delta,
        types::{ServerStartType, TestCase, TestError, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{
        SemanticToken, SemanticTokens, SemanticTokensDelta, SemanticTokensFullDeltaResult,
        SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
        SemanticTokensServerCapabilities, ServerCapabilities, Uri, WorkDoneProgressOptions,
    };
    use rstest::rstest;

    fn semantic_tokens_full_delta_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            semantic_tokens_provider: Some(
                SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: None,
                    },
                    legend: SemanticTokensLegend::default(),
                    range: Some(false),
                    full: Some(SemanticTokensFullOptions::Delta { delta: Some(true) }),
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
        // NOTE: We send a `199` here so that we receive a valid response for the initial
        // `textDocument/semanticTokens/full` response, but an empty response for the following
        // `textDocument/semanticTokens/full/delta` request
        send_response_num(199, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(
            &semantic_tokens_full_delta_capabilities_simple(),
            &test_case_root,
        )
        .expect("Failed to send capabilities");

        lspresso_shot!(test_semantic_tokens_full_delta(test_case, None));
    }

    #[rstest]
    fn test_server_simple_expect_none_got_some(
        #[values(
            100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, /*113, 114, 115*/
        )]
        response_num: u32,
    ) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp =
            test_server::responses::get_semantic_tokens_full_delta_response(response_num, &uri)
                .unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(
            &semantic_tokens_full_delta_capabilities_simple(),
            &test_case_root,
        )
        .expect("Failed to send capabilities");
        let test_result = test_semantic_tokens_full_delta(test_case.clone(), None);
        let expected_err = TestError::ExpectedNone(test_case.test_id, format!("{resp:#?}"));
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_simple_expect_some_got_some(
        #[values(
            100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 101, 112, /*113, 114, 115, 115,
            117, 118, 119*/
        )]
        response_num: u32,
    ) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp =
            test_server::responses::get_semantic_tokens_full_delta_response(response_num, &uri)
                .unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(
            &semantic_tokens_full_delta_capabilities_simple(),
            &test_case_root,
        )
        .expect("Failed to send capabilities");

        lspresso_shot!(test_semantic_tokens_full_delta(test_case, Some(&resp)));
    }

    #[ignore = "rust-analyzer behaves non-deterministically"]
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

        // These are the possible values returned...
        let _possible_expected = vec![
            SemanticTokensFullDeltaResult::TokensDelta(SemanticTokensDelta {
                result_id: Some("5".to_string()),
                edits: vec![],
            }),
            SemanticTokensFullDeltaResult::TokensDelta(SemanticTokensDelta {
                result_id: Some("6".to_string()),
                edits: vec![],
            }),
            SemanticTokensFullDeltaResult::Tokens(SemanticTokens {
                result_id: Some("5".to_string()),
                data: vec![
                    SemanticToken {
                        delta_line: 0,
                        delta_start: 7,
                        length: 4,
                        token_type: 4,
                        token_modifiers_bitset: 262_148,
                    },
                    SemanticToken {
                        delta_line: 1,
                        delta_start: 12,
                        length: 3,
                        token_type: 17,
                        token_modifiers_bitset: 4,
                    },
                ],
            }),
            SemanticTokensFullDeltaResult::Tokens(SemanticTokens {
                result_id: Some("6".to_string()),
                data: vec![
                    SemanticToken {
                        delta_line: 0,
                        delta_start: 7,
                        length: 4,
                        token_type: 4,
                        token_modifiers_bitset: 262_148,
                    },
                    SemanticToken {
                        delta_line: 1,
                        delta_start: 12,
                        length: 3,
                        token_type: 17,
                        token_modifiers_bitset: 4,
                    },
                ],
            }),
        ];
        lspresso_shot!(test_semantic_tokens_full_delta(test_case, None));
    }
}
