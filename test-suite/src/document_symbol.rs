#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, time::Duration};

    use crate::test_helpers::cargo_dot_toml;
    use lspresso_shot::{
        lspresso_shot, test_document_symbol,
        types::{ServerStartType, TestCase, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{
        DocumentSymbol, DocumentSymbolResponse, OneOf, Position, Range, ServerCapabilities,
        SymbolKind,
    };
    use rstest::rstest;

    fn document_symbol_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            document_symbol_provider: Some(OneOf::Left(true)),
            ..Default::default()
        }
    }

    #[rstest]
    fn test_server_document_symbol_simple(#[values(0, 1, 2, 3)] response_num: u32) {
        let syms = test_server::responses::get_document_symbol_response(response_num).unwrap();
        let source_file = TestFile::new(test_server::get_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file).cleanup(false);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&document_symbol_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_document_symbol(test_case, &syms,));
    }

    #[test]
    fn rust_analyzer_document_symbol() {
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
    let foo = 5;
}",
        );
        let doc_sym_test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(5).unwrap(),
                "rustAnalyzer/Indexing".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .cleanup(false)
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_document_symbol(
            doc_sym_test_case,
            #[allow(deprecated)]
            &DocumentSymbolResponse::Nested(vec![DocumentSymbol {
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
            }]),
        ));
    }
}
