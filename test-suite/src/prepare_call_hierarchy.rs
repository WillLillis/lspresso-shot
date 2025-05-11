#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, str::FromStr as _, time::Duration};

    use crate::test_helpers::{NON_RESPONSE_NUM, cargo_dot_toml};
    use lspresso_shot::{
        lspresso_shot, test_prepare_call_hierarchy,
        types::{ServerStartType, TestCase, TestError, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{
        CallHierarchyItem, CallHierarchyServerCapability, Position, Range, ServerCapabilities,
        SymbolKind, Uri,
    };
    use rstest::rstest;

    fn prepare_call_hierarchy_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            call_hierarchy_provider: Some(CallHierarchyServerCapability::Simple(true)),
            ..ServerCapabilities::default()
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
            &prepare_call_hierarchy_capabilities_simple(),
            &test_case_root,
        )
        .expect("Failed to send capabilities");

        lspresso_shot!(test_prepare_call_hierarchy(
            test_case,
            Position::default(),
            None,
            None
        ));
    }

    #[rstest]
    fn test_server_simple_expect_none_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp =
            test_server::responses::get_prepare_call_hierachy_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(
            &prepare_call_hierarchy_capabilities_simple(),
            &test_case_root,
        )
        .expect("Failed to send capabilities");

        let test_result =
            test_prepare_call_hierarchy(test_case.clone(), Position::default(), None, None);
        let expected_err = TestError::ExpectedNone(test_case.test_id, format!("{resp:#?}"));
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_simple_expect_some_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp =
            test_server::responses::get_prepare_call_hierachy_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(
            &prepare_call_hierarchy_capabilities_simple(),
            &test_case_root,
        )
        .expect("Failed to send capabilities");

        lspresso_shot!(test_prepare_call_hierarchy(
            test_case,
            Position::default(),
            None,
            Some(&resp)
        ));
    }

    #[test]
    fn rust_analyzer() {
        let source_file = TestFile::new(
            "src/main.rs",
            r#"pub fn main() {
    println!("Hello, world!");
}"#,
        );
        let test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(4).unwrap(),
                "rustAnalyzer/cachePriming".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_prepare_call_hierarchy(
            test_case,
            Position::new(0, 8),
            None,
            Some(&vec![CallHierarchyItem {
                name: "main".to_string(),
                kind: SymbolKind::FUNCTION,
                tags: None,
                detail: Some("pub fn main()".to_string()),
                uri: Uri::from_str("src/main.rs").unwrap(),
                range: Range {
                    start: Position::new(0, 0),
                    end: Position::new(2, 1),
                },
                selection_range: Range {
                    start: Position::new(0, 7),
                    end: Position::new(0, 11),
                },
                data: None,
            }])
        ));
    }
}
