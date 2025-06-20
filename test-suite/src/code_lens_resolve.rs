#[cfg(test)]
mod test {
    use std::str::FromStr as _;

    use crate::test_helpers::NON_RESPONSE_NUM;
    use lspresso_shot::{
        lspresso_shot, test_code_lens_resolve,
        types::{ResponseMismatchError, TestCase, TestError, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{CodeLens, CodeLensOptions, Range, ServerCapabilities, Uri};
    use rstest::rstest;

    fn code_lens_resolve_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            code_lens_provider: Some(CodeLensOptions {
                resolve_provider: Some(true),
            }),
            ..Default::default()
        }
    }

    fn get_dummy_code_lens(test_case: &TestCase) -> CodeLens {
        let uri = test_case.get_source_file_path("").unwrap();
        CodeLens {
            range: Range::default(),
            command: None,
            data: Some(serde_json::json!({ "uri": &uri })),
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
        send_capabiltiies(&code_lens_resolve_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");
        let code_lens = get_dummy_code_lens(&test_case);

        lspresso_shot!(test_code_lens_resolve(
            &test_case, None, &code_lens, None, None
        ));
    }

    #[rstest]
    fn test_server_simple_expect_none_got_some(#[values(0, 1)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp =
            test_server::responses::get_code_lens_resolve_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&code_lens_resolve_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");
        let code_lens = get_dummy_code_lens(&test_case);

        let test_result = test_code_lens_resolve(&test_case, None, &code_lens, None, None);
        let expected_err = TestError::ResponseMismatch(ResponseMismatchError {
            test_id: test_case.test_id,
            expected: None,
            actual: Some(resp),
        });
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_simple_expect_some_got_some(#[values(0, 1)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp =
            test_server::responses::get_code_lens_resolve_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&code_lens_resolve_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");
        let code_lens = get_dummy_code_lens(&test_case);

        lspresso_shot!(test_code_lens_resolve(
            &test_case,
            None,
            &code_lens,
            None,
            Some(&resp)
        ));
    }

    // NOTE: It's difficult to test `codeLens/resolve` requests with rust-analyzer, as its
    // responses contain JSON data that are specific to the ephemeral test directory
}
