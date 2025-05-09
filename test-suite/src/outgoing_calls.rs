#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, str::FromStr as _, time::Duration};

    use crate::test_helpers::{cargo_dot_toml, NON_RESPONSE_NUM};
    use lspresso_shot::{
        lspresso_shot, test_outgoing_calls,
        types::{ServerStartType, TestCase, TestError, TestFile},
    };
    use test_server::{
        get_dummy_server_path, get_dummy_source_path,
        responses::get_prepare_call_hierachy_response, send_capabiltiies, send_response_num,
    };

    use lsp_types::{
        CallHierarchyItem, CallHierarchyOutgoingCall, CallHierarchyServerCapability, Position,
        Range, ServerCapabilities, SymbolKind, Uri,
    };
    use rstest::rstest;

    fn outgoing_calls_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            call_hierarchy_provider: Some(CallHierarchyServerCapability::Simple(true)),
            ..ServerCapabilities::default()
        }
    }

    ///  Since since there isn't a `textDocument` field inside the `callHierarchy/outgoingCalls`
    ///  request params, we need to hack in a valid `uri` field so that the test server
    ///  can respond properly
    fn get_full_dummy_source_path(test_case: &TestCase) -> Uri {
        Uri::from_str(
            test_case
                .get_source_file_path(get_dummy_source_path())
                .unwrap()
                .to_str()
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn test_server_simple_expect_none_got_none() {
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(NON_RESPONSE_NUM, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&outgoing_calls_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_outgoing_calls(
            test_case.clone(),
            &CallHierarchyItem {
                name: "these fields".to_string(),
                kind: SymbolKind::EVENT,
                tags: None,
                detail: Some("don't matter".to_string()),
                uri: get_full_dummy_source_path(&test_case),
                range: Range::default(),
                selection_range: Range::default(),
                data: None
            },
            None,
            None
        ));
    }

    #[rstest]
    fn test_server_simple_expect_none_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp = test_server::responses::get_outgoing_calls_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&outgoing_calls_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        let mut call_item = get_prepare_call_hierachy_response(1, &uri).unwrap()[0].clone();
        call_item.uri = get_full_dummy_source_path(&test_case);
        let test_result = test_outgoing_calls(test_case.clone(), &call_item, None, None);
        let expected_err = TestError::ExpectedNone(test_case.test_id, format!("{resp:#?}"));
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_simple_expect_some_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp = test_server::responses::get_outgoing_calls_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&outgoing_calls_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        let mut call_item = get_prepare_call_hierachy_response(1, &uri).unwrap()[0].clone();
        call_item.uri = get_full_dummy_source_path(&test_case);
        lspresso_shot!(test_outgoing_calls(
            test_case,
            &call_item,
            None,
            Some(&resp)
        ));
    }

    #[test]
    fn rust_analyzer() {
        let source_file = TestFile::new(
            "src/main.rs",
            r#"fn foo() {}
pub fn main() {
    foo();
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

        let uri = Uri::from_str(&format!(
            "file://{}",
            test_case
                .get_source_file_path("src/main.rs")
                .unwrap()
                .to_str()
                .unwrap(),
        ))
        .unwrap();

        let call_item = CallHierarchyItem {
            name: "main".to_string(),
            kind: SymbolKind::FUNCTION,
            tags: None,
            detail: Some("pub fn main()".to_string()),
            uri,
            range: Range {
                start: Position::new(1, 0),
                end: Position::new(4, 1),
            },
            selection_range: Range {
                start: Position::new(1, 7),
                end: Position::new(1, 11),
            },
            data: None,
        };

        lspresso_shot!(test_outgoing_calls(
            test_case,
            &call_item,
            None,
            Some(&vec![CallHierarchyOutgoingCall {
                to: CallHierarchyItem {
                    name: "foo".to_string(),
                    kind: SymbolKind::FUNCTION,
                    tags: None,
                    detail: Some("fn foo()".to_string()),
                    uri: Uri::from_str("src/main.rs").unwrap(),
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: 0,
                            character: 11,
                        },
                    },
                    selection_range: Range {
                        start: Position {
                            line: 0,
                            character: 3,
                        },
                        end: Position {
                            line: 0,
                            character: 6,
                        },
                    },
                    data: None,
                },
                from_ranges: vec![Range {
                    start: Position {
                        line: 2,
                        character: 4,
                    },
                    end: Position {
                        line: 2,
                        character: 7,
                    },
                }]
            }])
        ));
    }
}
