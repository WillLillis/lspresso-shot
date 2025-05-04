#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, str::FromStr as _, time::Duration};

    use crate::test_helpers::{cargo_dot_toml, NON_RESPONSE_NUM};
    use lspresso_shot::{
        lspresso_shot, test_formatting,
        types::{formatting::FormattingResult, ServerStartType, TestCase, TestError, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{OneOf, Position, Range, ServerCapabilities, TextEdit, Uri};
    use rstest::rstest;

    fn formatting_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            document_formatting_provider: Some(OneOf::Left(true)),
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
        // since the start and end statesof the source file are the same
        send_response_num(NON_RESPONSE_NUM, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&formatting_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_formatting(
            test_case,
            None,
            Some(&FormattingResult::EndState(contents.to_string()))
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

        lspresso_shot!(test_formatting(test_case, None, None));
    }

    #[rstest]
    fn test_server_response_simple_expect_none_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
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

        let test_result = test_formatting(test_case.clone(), None, None);
        let expected_err = TestError::ExpectedNone(test_case.test_id, format!("{edits:#?}"));
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
            test_case,
            None,
            Some(&FormattingResult::Response(edits))
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
            test_case,
            None,
            Some(&FormattingResult::EndState(
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
            test_case,
            None,
            Some(&FormattingResult::Response(vec![
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
}
