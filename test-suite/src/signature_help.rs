#[cfg(test)]
mod test {
    use std::str::FromStr as _;

    use std::{num::NonZeroU32, time::Duration};

    use crate::test_helpers::{NON_RESPONSE_NUM, cargo_dot_toml};
    use lspresso_shot::{
        lspresso_shot, test_signature_help,
        types::{ServerStartType, TestCase, TestError, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{
        ParameterInformation, Position, ServerCapabilities, SignatureHelp, SignatureHelpOptions,
        SignatureInformation, Uri, WorkDoneProgressOptions,
    };
    use rstest::rstest;

    fn signature_help_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            signature_help_provider: Some(SignatureHelpOptions {
                trigger_characters: None,
                retrigger_characters: None,
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: None,
                },
            }),
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
        send_capabiltiies(&signature_help_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_signature_help(
            test_case,
            Position::default(),
            None,
            None,
            None
        ));
    }

    #[rstest]
    fn test_server_simple_expect_none_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp = test_server::responses::get_signature_help_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&signature_help_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        let expected_err = TestError::ExpectedNone(test_case.test_id.clone(), format!("{resp:#?}"));
        let test_result = test_signature_help(test_case, Position::default(), None, None, None);
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_simple_expect_some_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp = test_server::responses::get_signature_help_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&signature_help_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_signature_help(
            test_case,
            Position::default(),
            None,
            None,
            Some(&resp),
        ));
    }

    #[test]
    fn rust_analyzer() {
        let source_file = TestFile::new(
            "src/main.rs",
            r"fn foo(bar: i32) -> void {}
pub fn main() {
    foo()
}",
        );
        let test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(4).unwrap(),
                "rustAnalyzer/cachePriming".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .other_file(cargo_dot_toml());

        lspresso_shot!(test_signature_help(
            test_case,
            Position::new(2, 8),
            None,
            None,
            Some(&SignatureHelp {
                signatures: vec![SignatureInformation {
                    label: "fn foo(bar: i32) -> {unknown}".to_string(),
                    documentation: None,
                    parameters: Some(vec![ParameterInformation {
                        label: lsp_types::ParameterLabel::LabelOffsets([7, 15]),
                        documentation: None,
                    }]),
                    active_parameter: Some(0),
                },],
                active_signature: Some(0),
                active_parameter: Some(0),
            })
        ));
    }
}
