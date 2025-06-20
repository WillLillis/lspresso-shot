#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, str::FromStr as _, time::Duration};

    use crate::test_helpers::{NON_RESPONSE_NUM, cargo_dot_toml};
    use lspresso_shot::{
        lspresso_shot, test_formatting, test_on_type_formatting, test_range_formatting,
        types::{
            ResponseMismatchError, ServerStartType, StateOrResponse, TestCase, TestError, TestFile,
        },
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{
        DocumentOnTypeFormattingOptions, OneOf, Position, Range, ServerCapabilities, TextEdit, Uri,
    };
    use rstest::rstest;

    fn formatting_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            document_formatting_provider: Some(OneOf::Left(true)),
            ..Default::default()
        }
    }

    fn range_formatting_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            document_range_formatting_provider: Some(OneOf::Left(true)),
            ..Default::default()
        }
    }

    fn on_type_formatting_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            document_on_type_formatting_provider: Some(DocumentOnTypeFormattingOptions {
                first_trigger_character: String::new(),
                more_trigger_character: None,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn test_server_state_simple_expect_some_got_some() {
        let contents = "Some source contents";
        let source_file = TestFile::new(test_server::get_dummy_source_path(), contents);
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        // NOTE: Sending a `None` empty edit response simplifies things here,
        // since the start and end states of the source file are the same
        send_response_num(NON_RESPONSE_NUM, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&formatting_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_formatting(
            &test_case,
            None,
            None,
            Some(&StateOrResponse::State(contents.to_string()))
        ));
    }

    #[test]
    fn test_server_response_simple_expect_none_got_none() {
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(NON_RESPONSE_NUM, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&formatting_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_formatting(&test_case, None, None, None));
    }

    #[test]
    fn test_server_on_type_simple_expect_none_got_none() {
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(NON_RESPONSE_NUM, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&on_type_formatting_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_on_type_formatting(
            &test_case,
            Position::default(),
            "",
            None,
            None,
            None
        ));
    }

    #[test]
    fn test_server_range_simple_expect_none_got_none() {
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(NON_RESPONSE_NUM, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&range_formatting_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_range_formatting(
            &test_case,
            Range::default(),
            None,
            None,
            None,
        ));
    }

    #[rstest]
    fn test_server_response_simple_expect_none_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp = StateOrResponse::Response(
            test_server::responses::get_formatting_response(response_num, &uri).unwrap(),
        );
        let source_file =
            TestFile::new(test_server::get_dummy_source_path(), "Some source contents");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&formatting_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        let test_result = test_formatting(&test_case, None, None, None);
        let expected_err = TestError::ResponseMismatch(ResponseMismatchError {
            test_id: test_case.test_id,
            expected: None,
            actual: Some(resp),
        });
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_on_type_simple_expect_none_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp =
            test_server::responses::get_on_type_formatting_response(response_num, &uri).unwrap();
        let source_file =
            TestFile::new(test_server::get_dummy_source_path(), "Some source contents");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&on_type_formatting_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        let test_result =
            test_on_type_formatting(&test_case, Position::default(), "", None, None, None);
        let expected_err = TestError::ResponseMismatch(ResponseMismatchError {
            test_id: test_case.test_id,
            expected: None,
            actual: Some(resp),
        });
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_response_simple_expect_some_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let edits = test_server::responses::get_formatting_response(response_num, &uri).unwrap();
        let source_file =
            TestFile::new(test_server::get_dummy_source_path(), "Some source contents");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&formatting_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_formatting(
            &test_case,
            None,
            None,
            Some(&StateOrResponse::Response(edits))
        ));
    }

    #[rstest]
    fn test_server_range_simple_expect_some_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let edits =
            test_server::responses::get_formatting_range_response(response_num, &uri).unwrap();
        let source_file =
            TestFile::new(test_server::get_dummy_source_path(), "Some source contents");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&range_formatting_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_range_formatting(
            &test_case,
            Range::default(),
            None,
            None,
            Some(&edits)
        ));
    }

    #[rstest]
    fn test_server_on_type_simple_expect_some_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let edits =
            test_server::responses::get_on_type_formatting_response(response_num, &uri).unwrap();
        let source_file =
            TestFile::new(test_server::get_dummy_source_path(), "Some source contents");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&on_type_formatting_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_on_type_formatting(
            &test_case,
            Position::default(),
            "",
            None,
            None,
            Some(&edits)
        ));
    }

    #[test]
    fn rust_analyzer_state() {
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
let foo = 5;
}",
        );
        let test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(1).unwrap(),
                "rustAnalyzer/cachePriming".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_formatting(
            &test_case,
            None,
            None,
            Some(&StateOrResponse::State(
                "pub fn main() {
    let foo = 5;
}
"
                .to_string()
            ))
        ));
    }

    #[test]
    fn rust_analyzer_response() {
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
let foo = 5;
}",
        );
        let test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(1).unwrap(),
                "rustAnalyzer/cachePriming".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_formatting(
            &test_case,
            None,
            None,
            Some(&StateOrResponse::Response(vec![
                TextEdit {
                    new_text: "    ".to_string(),
                    range: Range {
                        start: Position::new(1, 0),
                        end: Position::new(1, 0),
                    },
                },
                TextEdit {
                    new_text: "\n".to_string(),
                    range: Range {
                        start: Position::new(2, 1),
                        end: Position::new(2, 1),
                    }
                }
            ])),
        ));
    }

    // NOTE: rust-analyzer doesn't support `textDocument/rangeFormatting` requests

    // With help from https://github.com/rust-lang/rust-analyzer/issues/16192
    #[test]
    fn rust_analyzer_on_type() {
        let source_file = TestFile::new(
            "src/main.rs",
            "fn main() {
    let greeting = \"Hello, World\"
}",
        );
        let test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(1).unwrap(),
                "rustAnalyzer/cachePriming".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_on_type_formatting(
            &test_case,
            Position::new(1, 18),
            "=",
            None,
            None,
            Some(&vec![TextEdit {
                range: Range {
                    start: Position::new(1, 33),
                    end: Position::new(1, 33),
                },
                new_text: ";".to_string(),
            }]),
        ));
    }
}
