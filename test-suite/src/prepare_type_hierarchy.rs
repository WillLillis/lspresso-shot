#[cfg(test)]
mod test {
    use crate::test_helpers::NON_RESPONSE_NUM;
    use lspresso_shot::{
        lspresso_shot, test_prepare_type_hierarchy,
        types::{TestCase, TestError, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{OneOf, Position, ServerCapabilities};
    use rstest::rstest;

    fn prepare_type_hierarchy_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            // need update from lsp-types once https://github.com/gluon-lang/lsp-types/pull/298 is
            // merged
            type_hierarchy_provider: Some(OneOf::Left(true)),
            ..ServerCapabilities::default()
        }
    }

    #[test]
    fn test_server_prepare_type_hierarchy_simple_expect_none_got_none() {
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file)
            .cursor_pos(Some(Position::default()));

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(NON_RESPONSE_NUM, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(
            &prepare_type_hierarchy_capabilities_simple(),
            &test_case_root,
        )
        .expect("Failed to send capabilities");

        lspresso_shot!(test_prepare_type_hierarchy(test_case, None));
    }

    #[rstest]
    fn test_server_prepare_type_hierarchy_simple_expect_none_got_some(
        #[values(0, 1, 2, 3)] response_num: u32,
    ) {
        let resp =
            test_server::responses::get_prepare_type_hierachy_response(response_num).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file)
            .cursor_pos(Some(Position::default()));

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(
            &prepare_type_hierarchy_capabilities_simple(),
            &test_case_root,
        )
        .expect("Failed to send capabilities");

        let test_result = test_prepare_type_hierarchy(test_case.clone(), None);
        let expected_err = TestError::ExpectedNone(test_case.test_id, format!("{resp:#?}"));
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_prepare_type_hierarchy_simple_expect_some_got_some(
        #[values(0, 1, 2, 3)] response_num: u32,
    ) {
        let resp =
            test_server::responses::get_prepare_type_hierachy_response(response_num).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file)
            .cursor_pos(Some(Position::default()));

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(
            &prepare_type_hierarchy_capabilities_simple(),
            &test_case_root,
        )
        .expect("Failed to send capabilities");

        lspresso_shot!(test_prepare_type_hierarchy(test_case, Some(&resp)));
    }

    // NOTE: rust-analyzer doesn't support `textDocument/prepareTypeHierarchy` requests
}
