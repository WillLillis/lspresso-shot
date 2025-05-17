#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, str::FromStr as _, time::Duration};

    use crate::test_helpers::{NON_RESPONSE_NUM, cargo_dot_toml};
    use lspresso_shot::{
        lspresso_shot, test_semantic_tokens_range,
        types::{ResponseMismatchError, ServerStartType, TestCase, TestError, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{
        Position, Range, SemanticToken, SemanticTokens, SemanticTokensLegend,
        SemanticTokensOptions, SemanticTokensPartialResult, SemanticTokensRangeResult,
        SemanticTokensServerCapabilities, ServerCapabilities, Uri, WorkDoneProgressOptions,
    };
    use rstest::rstest;

    fn semantic_tokens_range_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            semantic_tokens_provider: Some(
                SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: None,
                    },
                    legend: SemanticTokensLegend::default(),
                    range: Some(true),
                    full: None,
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
        send_capabiltiies(
            &semantic_tokens_range_capabilities_simple(),
            &test_case_root,
        )
        .expect("Failed to send capabilities");

        lspresso_shot!(test_semantic_tokens_range(
            test_case,
            Range::default(),
            None,
            None
        ));
    }

    #[rstest]
    fn test_server_simple_expect_none_got_some(
        #[values(0, 1, 2, 3, 4, 5, 6, 7, 8)] response_num: u32,
    ) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp =
            test_server::responses::get_semantic_tokens_range_response(response_num, &uri).unwrap();
        let resp_data = match &resp {
            SemanticTokensRangeResult::Tokens(SemanticTokens { data, .. }) => data.clone(),
            SemanticTokensRangeResult::Partial(SemanticTokensPartialResult { data }) => {
                data.clone()
            }
        };
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(
            &semantic_tokens_range_capabilities_simple(),
            &test_case_root,
        )
        .expect("Failed to send capabilities");
        let test_result =
            test_semantic_tokens_range(test_case.clone(), Range::default(), None, None);
        #[allow(clippy::useless_let_if_seq)]
        let mut expected_err = TestError::ResponseMismatch(ResponseMismatchError {
            test_id: test_case.test_id.clone(),
            expected: None,
            actual: Some(resp),
        });

        // HACK: Because of the serialization issues with `SemanticTokensRangeResult`,
        // we have to work around
        if (5..=8).contains(&response_num) {
            expected_err = TestError::ResponseMismatch(ResponseMismatchError {
                test_id: test_case.test_id,
                expected: None,
                actual: Some(SemanticTokensRangeResult::Tokens(SemanticTokens {
                    result_id: None,
                    data: resp_data,
                })),
            });
        }
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_simple_expect_some_got_some(
        #[values(0, 1, 2, 3, 4, 5, 6, 7, 8)] response_num: u32,
    ) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp =
            test_server::responses::get_semantic_tokens_range_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(
            &semantic_tokens_range_capabilities_simple(),
            &test_case_root,
        )
        .expect("Failed to send capabilities");

        lspresso_shot!(test_semantic_tokens_range(
            test_case,
            Range::default(),
            None,
            Some(&resp)
        ));
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
        let possible_results = vec![
            SemanticTokensRangeResult::Tokens(SemanticTokens {
                result_id: Some("3".to_string()),
                data: vec![SemanticToken {
                    delta_line: 0,
                    delta_start: 7,
                    length: 4,
                    token_type: 4,
                    token_modifiers_bitset: 262_148,
                }],
            }),
            SemanticTokensRangeResult::Tokens(SemanticTokens {
                result_id: Some("4".to_string()),
                data: vec![SemanticToken {
                    delta_line: 0,
                    delta_start: 7,
                    length: 4,
                    token_type: 4,
                    token_modifiers_bitset: 262_148,
                }],
            }),
            SemanticTokensRangeResult::Tokens(SemanticTokens {
                result_id: Some("5".to_string()),
                data: vec![SemanticToken {
                    delta_line: 0,
                    delta_start: 7,
                    length: 4,
                    token_type: 4,
                    token_modifiers_bitset: 262_148,
                }],
            }),
        ];
        let range = Range {
            start: Position::new(0, 7),
            end: Position::new(0, 10),
        };
        for result in &possible_results {
            if test_semantic_tokens_range(test_case.clone(), range, None, Some(result)).is_ok() {
                return;
            }
        }
        lspresso_shot!(test_semantic_tokens_range(
            test_case,
            range,
            None,
            Some(&possible_results[1]),
        ));
    }
}
