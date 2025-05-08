#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, str::FromStr as _, time::Duration};

    use crate::test_helpers::{cargo_dot_toml, NON_RESPONSE_NUM};
    use lspresso_shot::{
        lspresso_shot, test_code_lens,
        types::{ServerStartType, TestCase, TestError, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{CodeLens, CodeLensOptions, Position, Range, ServerCapabilities, Uri};
    use rstest::rstest;

    fn code_lens_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            code_lens_provider: Some(CodeLensOptions {
                resolve_provider: Some(false),
            }),
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
        send_capabiltiies(&code_lens_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_code_lens(test_case, None, None, None));
    }

    #[rstest]
    fn test_server_simple_expect_none_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp = test_server::responses::get_code_lens_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&code_lens_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        let test_result = test_code_lens(test_case.clone(), None, None, None);
        let expected_err = TestError::ExpectedNone(test_case.test_id, format!("{resp:#?}"));
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_simple_expect_some_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp = test_server::responses::get_code_lens_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&code_lens_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_code_lens(test_case, None, None, Some(&resp)));
    }

    // NOTE: It's difficult to test for equality with rust-analyzer here, as part
    // of the response contains arbitrary JSON values.
    #[test]
    fn rust_analyzer() {
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
    let mut foo = 5;
    foo = 10;
}",
        );
        let test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(4).unwrap(),
                "rustAnalyzer/cachePriming".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .other_file(cargo_dot_toml());

        let commands = vec![
            "rust-analyzer.runSingle".to_string(),
            "rust-analyzer.debugSingle".to_string(),
            "rust-analyzer.showReferences".to_string(),
            "rust-analyzer.gotoLocation".to_string(),
        ];
        let expected = vec![
            CodeLens {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 7,
                    },
                    end: Position {
                        line: 0,
                        character: 11,
                    },
                },
                command: None,
                data: None,
            },
            CodeLens {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 7,
                    },
                    end: Position {
                        line: 0,
                        character: 11,
                    },
                },
                command: None,
                data: None,
            },
        ];
        let cmp =
            |expected: &Vec<CodeLens>, actual: &Vec<CodeLens>, _test_case: &TestCase| -> bool {
                if expected.len() != actual.len() {
                    return false;
                }
                for (exp, act) in expected.iter().zip(actual.iter()) {
                    if exp.range != act.range {
                        return false;
                    }
                }
                true
            };

        lspresso_shot!(test_code_lens(
            test_case,
            Some(&commands),
            Some(cmp),
            Some(&expected)
        ));
    }
}
