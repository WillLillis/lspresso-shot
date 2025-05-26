#[cfg(test)]
mod test {
    use std::str::FromStr as _;

    use crate::test_helpers::NON_RESPONSE_NUM;
    use lspresso_shot::{
        lspresso_shot, test_workspace_execute_command,
        types::{TestCase, TestFile},
    };
    use test_server::{get_dummy_server_path, send_capabiltiies, send_response_num};

    use lsp_types::{ExecuteCommandOptions, ServerCapabilities, Uri};
    use rstest::rstest;

    fn execute_command_capabilities_simple(commands: &[String]) -> ServerCapabilities {
        ServerCapabilities {
            execute_command_provider: Some(ExecuteCommandOptions {
                commands: commands.to_vec(),
                ..Default::default()
            }),
            ..ServerCapabilities::default()
        }
    }

    #[test]
    fn test_server_simple_expect_none_got_none() {
        let commands = vec!["lspresso.testCommand".to_string()];
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        let uri = Uri::from_str(&format!(
            "file://{}",
            test_case
                .get_source_file_path("src/main.rs")
                .unwrap()
                .to_str()
                .unwrap(),
        ))
        .unwrap();
        let uri_val = serde_json::to_value(&uri).unwrap();
        send_response_num(NON_RESPONSE_NUM, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(
            &execute_command_capabilities_simple(&commands),
            &test_case_root,
        )
        .expect("Failed to send capabilities");

        lspresso_shot!(test_workspace_execute_command(
            &test_case,
            Some(&commands),
            "lspresso.testCommand",
            Some(&vec![uri_val]),
            None,
            None,
        ));
    }

    #[rstest]
    fn test_server_simple_expect_none_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
        use lspresso_shot::types::{ResponseMismatchError, TestError};

        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        let uri = Uri::from_str(&format!(
            "file://{}",
            test_case
                .get_source_file_path("src/main.rs")
                .unwrap()
                .to_str()
                .unwrap(),
        ))
        .unwrap();
        let uri_val = serde_json::to_value(&uri).unwrap();
        let resp =
            test_server::responses::get_execute_command_response(response_num, &uri).unwrap();
        let commands = vec!["lspresso.testCommand".to_string()];
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(
            &execute_command_capabilities_simple(&commands),
            &test_case_root,
        )
        .expect("Failed to send capabilities");
        let test_result = test_workspace_execute_command(
            &test_case,
            Some(&commands),
            "lspresso.testCommand",
            Some(&vec![uri_val]),
            None,
            None,
        );
        let expected_err = TestError::ResponseMismatch(ResponseMismatchError {
            test_id: test_case.test_id,
            expected: None,
            actual: Some(resp),
        });
        assert_eq!(Err(expected_err), test_result);
    }

    #[rstest]
    fn test_server_simple_expect_some_got_some(#[values(0, 1, 2, 3)] response_num: u32) {
        let source_file = TestFile::new(test_server::get_dummy_source_path(), "");
        let test_case = TestCase::new(get_dummy_server_path(), source_file);
        let uri = Uri::from_str(&format!(
            "file://{}",
            test_case
                .get_source_file_path("src/main.rs")
                .unwrap()
                .to_str()
                .unwrap(),
        ))
        .unwrap();
        let uri_val = serde_json::to_value(&uri).unwrap();

        let resp =
            test_server::responses::get_execute_command_response(response_num, &uri).unwrap();

        let commands = vec!["lspresso.testCommand".to_string()];
        let test_case_root = test_case
            .get_lspresso_dir()
            .expect("Failed to get test case's root directory");
        send_response_num(response_num, &test_case_root).expect("Failed to send response num");
        send_capabiltiies(
            &execute_command_capabilities_simple(&commands),
            &test_case_root,
        )
        .expect("Failed to send capabilities");

        lspresso_shot!(test_workspace_execute_command(
            &test_case,
            Some(&commands),
            "lspresso.testCommand",
            Some(&vec![uri_val]),
            None,
            Some(&resp),
        ));
    }

    // NOTE: rust-analzyer doesn't support `workspace/executeCommand`?
}
