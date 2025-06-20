#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, str::FromStr as _, time::Duration};

    use crate::test_helpers::{NON_RESPONSE_NUM, cargo_dot_toml};
    use lsp_types::{
        Location, OneOf, Position, Range, ServerCapabilities, SymbolInformation, SymbolKind,
        SymbolTag, Uri, WorkDoneProgressOptions, WorkspaceSymbolOptions, WorkspaceSymbolResponse,
    };
    use lspresso_shot::{
        lspresso_shot, test_workspace_symbol, test_workspace_symbol_resolve,
        types::{
            CleanResponse as _, ResponseMismatchError, ServerStartType, TestCase, TestError,
            TestFile,
        },
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use rstest::rstest;

    fn workspace_symbol_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            workspace_symbol_provider: Some(OneOf::Left(true)),
            ..ServerCapabilities::default()
        }
    }

    fn workspace_symbol_resolve_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            workspace_symbol_provider: Some(OneOf::Right(WorkspaceSymbolOptions {
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: None,
                },
                resolve_provider: Some(true),
            })),
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
    fn test_server_simple_expect_none_got_none() {
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        let uri = test_case
            .get_source_file_path("")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        send_response_num(NON_RESPONSE_NUM, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&workspace_symbol_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_workspace_symbol(&test_case, &uri, None, None));
    }

    #[rstest]
    fn test_server_simple_expect_none_got_some(
        #[values(0, 1, 2, 3, 4, 5, 6, 7)] response_num: u32,
    ) {
        // NOTE: The URI passed here matches the cleaned URI in the response
        let resp = test_server::responses::get_workspace_symbol_response(
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
        send_capabiltiies(&workspace_symbol_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");
        let uri = get_dummy_uri(&test_case);

        let test_result = test_workspace_symbol(&test_case, &uri, None, None);
        let resp = WorkspaceSymbolResponse::clean_response(resp, &test_case).unwrap();
        let mut expected_err = TestError::ResponseMismatch(ResponseMismatchError {
            test_id: test_case.test_id.clone(),
            expected: None,
            actual: Some(resp),
        });
        match response_num {
            // HACK: Because of the serialization issues with `WorkspaceSymbolResponse`, we have
            // to work around
            1 => {
                expected_err = TestError::ResponseMismatch(ResponseMismatchError {
                    test_id: test_case.test_id,
                    expected: None,
                    actual: Some(WorkspaceSymbolResponse::Flat(vec![])),
                });
            }
            #[allow(deprecated)]
            5 => {
                expected_err = TestError::ResponseMismatch(ResponseMismatchError {
                    test_id: test_case.test_id,
                    expected: None,
                    actual: Some(WorkspaceSymbolResponse::Flat(vec![SymbolInformation {
                        name: "name1".to_string(),
                        kind: SymbolKind::FILE,
                        tags: Some(vec![SymbolTag::DEPRECATED]),
                        container_name: None,
                        location: Location {
                            uri: Uri::from_str("main.dummy").unwrap(),
                            range: Range::default(),
                        },
                        deprecated: None,
                    }])),
                });
            }
            _ => {}
        }
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_simple_expect_some_got_some(
        #[values(0, 1, 2, 3, 4, 5, 6, 7)] response_num: u32,
    ) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp =
            test_server::responses::get_workspace_symbol_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&workspace_symbol_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");
        let uri = get_dummy_uri(&test_case);

        lspresso_shot!(test_workspace_symbol(&test_case, &uri, None, Some(&resp)));
    }

    #[rstest]
    fn test_server_resolve_simple_expect_some_got_some(#[values(0, 1)] response_num: u32) {
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let uri = get_dummy_uri(&test_case);
        let resp = test_server::responses::get_workspace_symbol_resolve_response(
            response_num,
            &Uri::from_str(&uri).unwrap(),
        )
        .unwrap();

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(
            &workspace_symbol_resolve_capabilities_simple(),
            &test_case_root,
        )
        .expect("Failed to send capabilities");
        let cleaned_resp = resp.clone().clean_response(&test_case).unwrap();

        lspresso_shot!(test_workspace_symbol_resolve(
            &test_case,
            &resp,
            None,
            &cleaned_resp
        ));
    }

    #[test]
    fn rust_analyzer() {
        let source_file = TestFile::new("src/main.rs", "pub fn main() {}");
        let test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(4).unwrap(),
                "rustAnalyzer/cachePriming".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_workspace_symbol(
            &test_case,
            "",
            None,
            #[allow(deprecated)]
            Some(&WorkspaceSymbolResponse::Flat(vec![SymbolInformation {
                name: "main".to_string(),
                kind: SymbolKind::FUNCTION,
                tags: None,
                container_name: None,
                location: Location {
                    uri: Uri::from_str("src/main.rs").unwrap(),
                    range: Range::new(Position::new(0, 7), Position::new(0, 11)),
                },
                deprecated: None,
            }]))
        ));
    }

    // NOTE: It would be a pain to add a rust-analyzer test for `workspaceSymbol/resolve`,
    // since we need to pass in in a valid `WorkspaceSymbol`
}
