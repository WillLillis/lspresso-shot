#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, time::Duration};

    use crate::test_helpers::{cargo_dot_toml, NON_RESPONSE_NUM};
    use lspresso_shot::{
        lspresso_shot, test_semantic_tokens_full,
        types::{ServerStartType, TestCase, TestError, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{
        SemanticToken, SemanticTokens, SemanticTokensFullOptions, SemanticTokensLegend,
        SemanticTokensOptions, SemanticTokensServerCapabilities, ServerCapabilities,
        WorkDoneProgressOptions,
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
    fn test_server_semantic_tokens_full_simple_expect_none_got_none() {
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(NON_RESPONSE_NUM, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&semantic_tokens_full_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_semantic_tokens_full(test_case, None));
    }

    #[rstest]
    fn test_server_semantic_tokens_full_simple_expect_none_got_some(
        #[values(0, 1, 2, 3, 4, 5, 6, 7)] response_num: u32,
    ) {
        let resp = test_server::responses::get_semantic_tokens_full_response(response_num).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&semantic_tokens_full_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        let test_result = test_semantic_tokens_full(test_case.clone(), None);
        let expected_err = TestError::ExpectedNone(test_case.test_id, format!("{resp:#?}"));
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_semantic_tokens_full_simple_expect_some_got_some(
        #[values(0, 1, 2, 3, 4, 5, 6, 7)] response_num: u32,
    ) {
        let resp = test_server::responses::get_semantic_tokens_full_response(response_num).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&semantic_tokens_full_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_semantic_tokens_full(test_case, Some(&resp)));
    }

    #[test]
    fn rust_analyzer_selection_range() {
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
    let foo = 5;
}",
        );
        let test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(5).unwrap(),
                "rustAnalyzer/Indexing".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_semantic_tokens_full(
            test_case,
            Some(&SemanticTokens {
                result_id: Some("4".to_string()),
                data: vec![
                    SemanticToken {
                        delta_line: 0,
                        delta_start: 7,
                        length: 4,
                        token_type: 4,
                        token_modifiers_bitset: 262_148
                    },
                    SemanticToken {
                        delta_line: 1,
                        delta_start: 8,
                        length: 3,
                        token_type: 17,
                        token_modifiers_bitset: 4
                    },
                ]
            })
        ));
    }
}
