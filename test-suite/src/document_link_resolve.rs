#[cfg(test)]
mod test {
    use std::str::FromStr as _;

    use crate::test_helpers::NON_RESPONSE_NUM;
    use lspresso_shot::{
        lspresso_shot, test_document_link_resolve,
        types::{TestCase, TestError, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{
        DocumentLink, DocumentLinkOptions, Position, Range, ServerCapabilities, Uri,
        WorkDoneProgressOptions,
    };
    use rstest::rstest;

    fn document_link_resolve_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            document_link_provider: Some(DocumentLinkOptions {
                resolve_provider: Some(true),
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: None,
                },
            }),
            ..ServerCapabilities::default()
        }
    }

    #[test]
    fn test_server_document_link_resolve_simple_expect_none_got_none() {
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(NON_RESPONSE_NUM, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(
            &document_link_resolve_capabilities_simple(),
            &test_case_root,
        )
        .expect("Failed to send capabilities");

        let link = DocumentLink {
            range: Range::default(),
            target: Some(
                Uri::from_str(
                    test_case
                        .get_source_file_path("")
                        .unwrap()
                        .to_string_lossy()
                        .as_ref(),
                )
                .unwrap(),
            ),
            tooltip: None,
            data: None,
        };

        lspresso_shot!(test_document_link_resolve(test_case, &link, None));
    }

    #[rstest]
    fn test_server_document_link_resolve_simple_expect_none_got_some(
        #[values(0, 1, 2)] response_num: u32,
    ) {
        let resp =
            test_server::responses::get_document_link_resolve_response(response_num).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file)
            .cursor_pos(Some(Position::default()));

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(
            &document_link_resolve_capabilities_simple(),
            &test_case_root,
        )
        .expect("Failed to send capabilities");

        let link = DocumentLink {
            range: Range::default(),
            target: Some(
                Uri::from_str(
                    test_case
                        .get_source_file_path("")
                        .unwrap()
                        .to_string_lossy()
                        .as_ref(),
                )
                .unwrap(),
            ),
            tooltip: None,
            data: None,
        };

        let test_result = test_document_link_resolve(test_case.clone(), &link, None);
        let expected_err = TestError::ExpectedNone(test_case.test_id, format!("{resp:#?}"));
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_document_link_simple_expect_some_got_some(#[values(0, 1, 2)] response_num: u32) {
        let resp =
            test_server::responses::get_document_link_resolve_response(response_num).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(
            &document_link_resolve_capabilities_simple(),
            &test_case_root,
        )
        .expect("Failed to send capabilities");

        let link = DocumentLink {
            range: Range::default(),
            target: Some(
                Uri::from_str(
                    test_case
                        .get_source_file_path("")
                        .unwrap()
                        .to_string_lossy()
                        .as_ref(),
                )
                .unwrap(),
            ),
            tooltip: None,
            data: None,
        };

        lspresso_shot!(test_document_link_resolve(test_case, &link, Some(&resp)));
    }

    // NOTE: rust-analyzer doesn't support `documentLink/resolve`
}
