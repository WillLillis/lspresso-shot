#[cfg(test)]
mod tests {
    use std::{num::NonZeroU32, str::FromStr as _, time::Duration};

    use crate::test_helpers::cargo_dot_toml;
    use lspresso_shot::{
        lspresso_shot, test_diagnostics,
        types::{ServerStartType, TestCase, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{
        CodeDescription, Diagnostic, DiagnosticOptions, DiagnosticRelatedInformation,
        DiagnosticServerCapabilities, DiagnosticSeverity, DiagnosticTag, Location, NumberOrString,
        Position, Range, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
        WorkDoneProgressOptions,
    };
    use rstest::rstest;
    use serde_json::Map;

    fn diagnostic_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            diagnostic_provider: Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
                identifier: Some("test-server".to_string()),
                inter_file_dependencies: false,
                workspace_diagnostics: false,
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: None,
                },
            })),
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            ..Default::default()
        }
    }

    #[rstest]
    fn test_server_diagnostics(#[values(0, 1, 2)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_source_path()).unwrap();
        let resp = test_server::responses::get_diagnostics_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file)
            .cursor_pos(Some(Position::default()));
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&diagnostic_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_diagnostics(test_case, &resp.diagnostics));
    }

    #[test]
    fn rust_analyzer_multi_diagnostics() {
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
    let bar = 1;
}",
        );
        let diagnostic_test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(2).unwrap(),
                String::new(),
            ))
            .timeout(Duration::from_secs(20))
            .other_file(cargo_dot_toml());

        let mut data_map = Map::new();
        data_map.insert(
            "rendered".to_string(),
            serde_json::Value::String("warning: unused variable: `bar`\n --> src/main.rs:2:9\n  |\n2 |     let bar = 1;\n  |         ^^^ help: if this is intentional, prefix it with an underscore: `_bar`\n  |\n  = note: `#[warn(unused_variables)]` on by default\n\n".to_string()),
        );
        let uri = Uri::from_str("src/main.rs").unwrap();
        let range = Range {
            start: Position {
                line: 1,
                character: 8,
            },
            end: Position {
                line: 1,
                character: 11,
            },
        };
        lspresso_shot!(test_diagnostics(
            diagnostic_test_case,
            &vec![
                Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::WARNING),
                    code: Some(NumberOrString::String("unused_variables".to_string())),
                    code_description: None,
                    source: Some("rustc".to_string()),
                    message: "unused variable: `bar`\n`#[warn(unused_variables)]` on by default"
                        .to_string(),
                    related_information: Some(vec![DiagnosticRelatedInformation {
                        location: Location {
                            uri: uri.clone(),
                            range,
                        },
                        message: "if this is intentional, prefix it with an underscore: `_bar`"
                            .to_string(),
                    }]),
                    tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                    data: Some(serde_json::Value::Object(data_map)),
                },
                Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::HINT),
                    code: Some(NumberOrString::String("unused_variables".to_string())),
                    code_description: None,
                    source: Some("rustc".to_string()),
                    message: "if this is intentional, prefix it with an underscore: `_bar`"
                        .to_string(),
                    related_information: Some(vec![DiagnosticRelatedInformation {
                        location: Location { uri, range },
                        message: "original diagnostic".to_string(),
                    }]),
                    tags: None,
                    data: None,
                }
            ],
        ));
    }

    // NOTE:: Specifying the start type is ignored for diagnostics tests
    #[test]
    fn rust_analyzer_diagnostics() {
        let source_file = TestFile::new(
            "src/main.rs",
            r#"pub fn main() {
    println!("Hello, world!
}"#,
        );
        let diagnostic_test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(2).unwrap(),
                String::new(),
            ))
            .timeout(Duration::from_secs(20))
            .other_file(cargo_dot_toml());

        let mut data_map = Map::new();
        _ = data_map.insert(
            "rendered".to_string(),
            serde_json::Value::String("error[E0765]: unterminated double quote string\n --> src/main.rs:2:14\n  |\n2 |       println!(\"Hello, world!\n  |  ______________^\n3 | | }\n  | |_^\n\n".to_string()),
        );
        lspresso_shot!(test_diagnostics(
            diagnostic_test_case,
            &vec![Diagnostic {
                range: Range {
                    start: Position {
                        line: 1,
                        character: 13,
                    },
                    end: Position {
                        line: 2,
                        character: 1,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(NumberOrString::String("E0765".to_string())),
                code_description: Some(CodeDescription {
                    href: Uri::from_str("https://doc.rust-lang.org/error-index.html#E0765")
                        .unwrap()
                }),
                source: Some("rustc".to_string()),
                message: "unterminated double quote string".to_string(),
                related_information: None,
                tags: None,
                data: Some(serde_json::Value::Object(data_map)),
            }],
        ));
    }
}
