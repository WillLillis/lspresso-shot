#[cfg(test)]
mod test {
    use std::str::FromStr as _;

    use crate::test_helpers::NON_RESPONSE_NUM;
    use lsp_types::{
        CreateFilesParams, FileCreate, FileOperationRegistrationOptions, ServerCapabilities, Uri,
        WorkspaceEdit, WorkspaceFileOperationsServerCapabilities, WorkspaceServerCapabilities,
    };
    use lspresso_shot::{
        lspresso_shot, test_workspace_will_create_files,
        types::{CleanResponse as _, ResponseMismatchError, TestCase, TestError, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use rstest::rstest;

    fn workspace_will_create_files_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            workspace: Some(WorkspaceServerCapabilities {
                workspace_folders: None,
                file_operations: Some(WorkspaceFileOperationsServerCapabilities {
                    did_create: None,
                    will_create: Some(FileOperationRegistrationOptions { filters: vec![] }),
                    did_rename: None,
                    will_rename: None,
                    did_delete: None,
                    will_delete: None,
                }),
            }),
            ..ServerCapabilities::default()
        }
    }

    fn get_dummy_uri(test_case: &TestCase) -> String {
        test_case
            .get_source_file_path("main.dummy")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn test_server_create_simple_expect_none_got_none() {
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        let uri = get_dummy_uri(&test_case);
        send_response_num(NON_RESPONSE_NUM, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(
            &workspace_will_create_files_capabilities_simple(),
            &test_case_root,
        )
        .expect("Failed to send capabilities");
        let params = CreateFilesParams {
            files: vec![FileCreate { uri }],
        };

        lspresso_shot!(test_workspace_will_create_files(
            test_case, &params, None, None
        ));
    }

    #[rstest]
    fn test_server_create_simple_expect_none_got_some(
        #[values(0, 1, 2, 3, 4, 5)] response_num: u32,
    ) {
        // NOTE: The URI passed here matches the cleaned URI in the response
        let resp = test_server::responses::get_workspace_will_create_files_response(
            response_num,
            &Uri::from_str("main.dummy").unwrap(),
        )
        .unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(
            &workspace_will_create_files_capabilities_simple(),
            &test_case_root,
        )
        .expect("Failed to send capabilities");
        let uri = get_dummy_uri(&test_case);

        let params = CreateFilesParams {
            files: vec![FileCreate { uri }],
        };

        let test_result = test_workspace_will_create_files(test_case.clone(), &params, None, None);
        let resp = WorkspaceEdit::clean_response(resp, &test_case).unwrap();
        let expected_err = TestError::ResponseMismatch(ResponseMismatchError {
            test_id: test_case.test_id,
            expected: None,
            actual: Some(resp),
        });
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_create_simple_expect_some_got_some(
        #[values(0, 1, 2, 3, 4, 5)] response_num: u32,
    ) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp =
            test_server::responses::get_workspace_will_create_files_response(response_num, &uri)
                .unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(
            &workspace_will_create_files_capabilities_simple(),
            &test_case_root,
        )
        .expect("Failed to send capabilities");
        let uri = test_case
            .get_source_file_path("src/foo.rs")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let params = CreateFilesParams {
            files: vec![FileCreate { uri }],
        };

        lspresso_shot!(test_workspace_will_create_files(
            test_case,
            &params,
            None,
            Some(&resp)
        ));
    }

    // NOTE: rust-analyzer doesn't support `workspace/willCreateFiles` requests
}
