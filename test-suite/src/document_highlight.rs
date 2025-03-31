#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, str::FromStr as _, time::Duration};

    use crate::test_helpers::{cargo_dot_toml, NON_RESPONSE_NUM};
    use lspresso_shot::{
        lspresso_shot, test_document_highlight,
        types::{ServerStartType, TestCase, TestError, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{DocumentHighlight, OneOf, Position, Range, ServerCapabilities, Uri};
    use rstest::rstest;

    fn document_highlight_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            document_highlight_provider: Some(OneOf::Left(true)),
            ..ServerCapabilities::default()
        }
    }

    #[test]
    fn test_server_document_highlight_simple_expect_none_got_none() {
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(NON_RESPONSE_NUM, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&document_highlight_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_document_highlight(
            test_case,
            &Position::default(),
            None
        ));
    }

    #[rstest]
    fn test_server_document_highlight_simple_expect_none_got_some(
        #[values(0, 1, 2, 3)] response_num: u32,
    ) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp =
            test_server::responses::get_document_highlight_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&document_highlight_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        let test_result = test_document_highlight(test_case.clone(), &Position::default(), None);
        let expected_err = TestError::ExpectedNone(test_case.test_id, format!("{resp:#?}"));
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_document_highlight_simple_expect_some_got_some(
        #[values(0, 1, 2, 3)] response_num: u32,
    ) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp =
            test_server::responses::get_document_highlight_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&document_highlight_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_document_highlight(
            test_case,
            &Position::default(),
            Some(&resp)
        ));
    }

    #[test]
    fn rust_analyzer_document_highlight() {
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
    let foo = 10;
}",
        );
        let test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(5).unwrap(),
                "rustAnalyzer/Indexing".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_document_highlight(
            test_case,
            &Position::new(1, 9),
            Some(&vec![DocumentHighlight {
                range: Range {
                    start: Position::new(1, 8),
                    end: Position::new(1, 11),
                },
                kind: None,
            }])
        ));
    }
}
