#[cfg(test)]
mod test {
    use crate::test_helpers::{cargo_dot_toml, NON_RESPONSE_NUM};
    use lsp_types::{
        CodeAction, CodeActionContext, CodeActionKind, CodeActionOrCommand,
        CodeActionProviderCapability, CodeActionResponse, Position, Range, ServerCapabilities,
        TextEdit, WorkspaceEdit,
    };
    use lspresso_shot::{
        lspresso_shot, test_code_action,
        types::{ServerStartType, TestCase, TestFile},
    };
    use std::{collections::HashMap, num::NonZeroU32, str::FromStr as _, time::Duration};
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::Uri;
    use lspresso_shot::types::TestError;

    use rstest::rstest;

    fn code_action_capabilities_simple() -> ServerCapabilities {
        ServerCapabilities {
            code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
            ..ServerCapabilities::default()
        }
    }

    #[test]
    fn test_server_code_action_simple_expect_none_got_none() {
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(NON_RESPONSE_NUM, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&code_action_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_code_action(
            test_case,
            &Range::default(),
            &CodeActionContext::default(),
            None,
            None
        ));
    }

    #[rstest]
    fn test_server_code_action_simple_expect_none_got_some(
        #[values(0, 1, 2, 3)] response_num: u32,
    ) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp = test_server::responses::get_code_action_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&code_action_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        let test_result = test_code_action(
            test_case.clone(),
            &Range::default(),
            &CodeActionContext::default(),
            None,
            None,
        );
        let expected_err = TestError::ExpectedNone(test_case.test_id, format!("{resp:#?}"));
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_code_action_simple_expect_some_got_some(
        #[values(0, 1, 2, 3)] response_num: u32,
    ) {
        let uri = Uri::from_str(&test_server::get_dummy_source_path()).unwrap();
        let resp = test_server::responses::get_code_action_response(response_num, &uri).unwrap();
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);

        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(&code_action_capabilities_simple(), &test_case_root)
            .expect("Failed to send capabilities");

        lspresso_shot!(test_code_action(
            test_case,
            &Range::default(),
            &CodeActionContext::default(),
            None,
            Some(&resp)
        ));
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn rust_analyzer_code_action() {
        let source_file = TestFile::new(
            "src/main.rs",
            "pub fn main() {
    let x = 5;
}",
        );
        let test_case = TestCase::new("rust-analyzer", source_file)
            .start_type(ServerStartType::Progress(
                NonZeroU32::new(5).unwrap(),
                "rustAnalyzer/cachePriming".to_string(),
            ))
            .timeout(Duration::from_secs(20))
            .other_file(cargo_dot_toml());
        let range = Range::new(
            lsp_types::Position::new(1, 9),
            lsp_types::Position::new(1, 9),
        );

        let cmp = |expected: &CodeActionResponse,
                   actual: &CodeActionResponse,
                   test_case: &TestCase|
         -> bool {
            _ = test_case;
            if expected.len() != actual.len() {
                return false;
            }
            if expected == actual {
                return true;
            }
            for (exp, act) in expected.iter().zip(actual.iter()) {
                match (exp, act) {
                    (
                        CodeActionOrCommand::Command(expected_cmd),
                        CodeActionOrCommand::Command(actual_cmd),
                    ) => {
                        // it isn't feasible to clean up the arbitrary JSON data in
                        // the `arguments` field, so we just check if the others
                        // are equal
                        if expected_cmd.title != actual_cmd.title {
                            return false;
                        }
                        if expected_cmd.command != actual_cmd.command {
                            return false;
                        }
                    }
                    (
                        CodeActionOrCommand::CodeAction(expected_act),
                        CodeActionOrCommand::CodeAction(actual_act),
                    ) => {
                        // it isn't feasible to clean up the arbitrary JSON data in
                        // the `command` or `data` fields, so we just check if the others
                        // are equal
                        if expected_act.title != actual_act.title {
                            return false;
                        }
                        if expected_act.kind != actual_act.kind {
                            return false;
                        }
                        if expected_act.diagnostics != actual_act.diagnostics {
                            return false;
                        }
                        if expected_act.edit != actual_act.edit {
                            return false;
                        }
                        if expected_act.is_preferred != actual_act.is_preferred {
                            return false;
                        }
                        if expected_act.disabled != actual_act.disabled {
                            return false;
                        }
                    }
                    _ => return false,
                }
            }

            true
        };

        let mut changes: HashMap<Uri, Vec<TextEdit>> = HashMap::new();
        changes.insert(
            Uri::from_str("src/main.rs").unwrap(),
            vec![TextEdit {
                range: Range {
                    start: Position {
                        line: 1,
                        character: 8,
                    },
                    end: Position {
                        line: 1,
                        character: 9,
                    },
                },
                new_text: "_x".to_string(),
            }],
        );
        let edit = WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        };

        lspresso_shot!(test_code_action(
            test_case,
            &range,
            &CodeActionContext::default(),
            Some(cmp),
            Some(&vec![
                CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Insert explicit type `i32`".to_string(),
                    kind: Some(CodeActionKind::REFACTOR_REWRITE),
                    diagnostics: None,
                    edit: None,
                    command: None,
                    is_preferred: None,
                    disabled: None,
                    data: None // inequality ignored by `cmp`!
                }),
                CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Promote local to constant".to_string(),
                    kind: Some(CodeActionKind::REFACTOR),
                    diagnostics: None,
                    edit: None,
                    command: None,
                    is_preferred: None,
                    disabled: None,
                    data: None
                }),
                CodeActionOrCommand::CodeAction(CodeAction {
                    title: "if this is intentional, prefix it with an underscore: `_x`".to_string(),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: None,
                    edit: Some(edit),
                    command: None,
                    is_preferred: Some(false),
                    disabled: None,
                    data: None
                }),
            ]),
        ));
    }
}
