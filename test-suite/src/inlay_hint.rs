#[cfg(test)]
mod test {
    use std::{num::NonZeroU32, str::FromStr as _, time::Duration};

    use crate::test_helpers::{NON_RESPONSE_NUM, cargo_dot_toml};
    use lspresso_shot::{
        lspresso_shot, test_inlay_hint,
        types::{ResponseMismatchError, ServerStartType, TestCase, TestError, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{
        InlayHint, InlayHintKind, InlayHintLabel, OneOf, Position, Range, ServerCapabilities, Uri,
    };
    use rstest::rstest;

    fn inlay_hint_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            inlay_hint_provider: Some(OneOf::Left(true)),
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
        send_capabiltiies(&inlay_hint_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_inlay_hint(&test_case, Range::default(), None, None));
    }

    #[rstest]
    fn test_server_simple_expect_none_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp = test_server::responses::get_inlay_hint_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&inlay_hint_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        let test_result = test_inlay_hint(&test_case, Range::default(), None, None);
        let expected_err = TestError::ResponseMismatch(ResponseMismatchError {
            test_id: test_case.test_id,
            expected: None,
            actual: Some(resp),
        });
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_simple_expect_some_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp = test_server::responses::get_inlay_hint_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&inlay_hint_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_inlay_hint(
            &test_case,
            Range::default(),
            None,
            Some(&resp)
        ));
    }

    #[test]
    fn rust_analyzer() {
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
    let x = 1;
}",
        );
        let test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(4).unwrap(),
                "rustAnalyzer/cachePriming".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .other_file(cargo_dot_toml());

        let cmp = |expected: &Vec<InlayHint>, actual: &Vec<InlayHint>, test_case: &TestCase| {
            _ = test_case;
            if expected.len() != actual.len() {
                return false;
            }
            for (exp, act) in expected.iter().zip(actual.iter()) {
                if exp.position != act.position {
                    return false;
                }
                if exp.label != act.label {
                    return false;
                }
                if exp.kind != act.kind {
                    return false;
                }
                if exp.text_edits != act.text_edits {
                    return false;
                }
                if exp.tooltip != act.tooltip {
                    return false;
                }
                if exp.padding_left != act.padding_left {
                    return false;
                }
                if exp.padding_right != act.padding_right {
                    return false;
                }
                // ignore data
            }
            true
        };

        lspresso_shot!(test_inlay_hint(
            &test_case,
            Range::new(
                Position {
                    line: 0,
                    character: 0,
                },
                Position {
                    line: 2,
                    character: 1,
                },
            ),
            Some(cmp),
            Some(&vec![InlayHint {
                position: Position {
                    line: 1,
                    character: 9,
                },
                label: InlayHintLabel::String(": i32".to_string()),
                kind: Some(InlayHintKind::TYPE),
                text_edits: None,
                tooltip: None,
                padding_left: Some(false,),
                padding_right: Some(false,),
                data: None,
            }]),
        ));
    }
}
