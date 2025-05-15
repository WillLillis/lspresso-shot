#[cfg(test)]
mod test {
    use std::str::FromStr as _;

    use crate::test_helpers::NON_RESPONSE_NUM;
    use lspresso_shot::{
        lspresso_shot, test_semantic_tokens_full,
        types::{ResponseMismatchError, TestCase, TestError, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{
        SemanticTokens, SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
        SemanticTokensPartialResult, SemanticTokensResult, SemanticTokensServerCapabilities,
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
        let resp_data = match &resp {
            SemanticTokensResult::Tokens(SemanticTokens { data, .. })
            | SemanticTokensResult::Partial(SemanticTokensPartialResult { data }) => data.clone(),
        };
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&semantic_tokens_full_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");
        let test_result = test_semantic_tokens_full(test_case.clone(), None, None);
        let mut expected_err = TestError::ResponseMismatch(ResponseMismatchError {
            test_id: test_case.test_id.clone(),
            expected: None,
            actual: Some(resp),
        });
        match response_num {
            // HACK: Because of the serialization issues with `SemanticTokensResult`, we have
            // to work around
            8..=11 => {
                expected_err = TestError::ResponseMismatch(ResponseMismatchError {
                    test_id: test_case.test_id,
                    expected: None,
                    actual: Some(SemanticTokensResult::Tokens(SemanticTokens {
                        result_id: None,
                        data: resp_data,
                    })),
                });
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
}
