#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, str::FromStr as _, time::Duration};

    use crate::test_helpers::{NON_RESPONSE_NUM, cargo_dot_toml};
    use lspresso_shot::{
        lspresso_shot, test_document_symbol,
        types::{ServerStartType, TestCase, TestError, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{
        DocumentSymbol, DocumentSymbolResponse, OneOf, Position, Range, ServerCapabilities,
        SymbolKind, Uri,
    };
    use rstest::rstest;

    fn document_symbol_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            document_symbol_provider: Some(OneOf::Left(true)),
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
        send_capabiltiies(&document_symbol_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_document_symbol(test_case, None, None));
    }

    #[rstest]
    fn test_server_simple_expect_none_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let syms =
            test_server::responses::get_document_symbol_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&document_symbol_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        let test_result = test_document_symbol(test_case.clone(), None, None);
        let mut expected_err =
            TestError::ExpectedNone(test_case.test_id.clone(), format!("{syms:#?}"));
        if response_num == 1 {
            // HACK: Because of the deserialization issues with empty vector results,
            // this error rendered incorrectly as `Flat` rather than `Nested`
            assert_eq!(
                expected_err,
                TestError::ExpectedNone(
                    test_case.test_id.clone(),
                    "Nested(\n    [],\n)".to_string()
                )
            );
            expected_err =
                TestError::ExpectedNone(test_case.test_id, "Flat(\n    [],\n)".to_string());
        }
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_simple_expect_some_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let syms =
            test_server::responses::get_document_symbol_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&document_symbol_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_document_symbol(test_case, None, Some(&syms)));
    }

    #[test]
    fn rust_analyzer() {
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
    let foo = 5;
}",
        );
        let doc_sym_test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(4).unwrap(),
                "rustAnalyzer/cachePriming".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_document_symbol(
            doc_sym_test_case,
            None,
            #[allow(deprecated)]
            Some(&DocumentSymbolResponse::Nested(vec![DocumentSymbol {
                name: "main".to_string(),
                detail: Some("fn()".to_string()),
                kind: SymbolKind::FUNCTION,
                tags: Some(Vec::new()),
                deprecated: Some(false),
                range: Range {
                    start: Position::new(0, 0),
                    end: Position::new(2, 1),
                },
                selection_range: Range {
                    start: Position::new(0, 7),
                    end: Position::new(0, 11),
                },
                children: Some(vec![DocumentSymbol {
                    name: "foo".to_string(),
                    detail: None,
                    kind: SymbolKind::VARIABLE,
                    tags: Some(Vec::new()),
                    deprecated: Some(false),
                    range: Range {
                        start: Position::new(1, 4),
                        end: Position::new(1, 16),
                    },
                    selection_range: Range {
                        start: Position::new(1, 8),
                        end: Position::new(1, 11),
                    },
                    children: None,
                }]),
            }])),
        ));
    }
}
