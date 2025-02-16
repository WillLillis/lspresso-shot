use std::path::Path;

use crate::types::{ServerStartType, TestCase, TestType};

/// Construct the contents of an `init.lua` file to test an lsp request corresponding
/// to `init_type` using the given parameters
pub fn get_init_dot_lua(
    test_case: &TestCase,
    test_type: TestType,
    root_path: &Path,
    results_path: &Path,
    error_path: &Path,
    log_path: &Path,
    source_extension: &str,
) -> String {
    // Start out with some utilities, adding the relevant filetype and the attach
    // logic, and the relevant `check_progress_result` function to invoke our request
    // at the appropriate time
    let mut raw_init = format!(
        "{}{}{}",
        include_str!("lua_templates/helpers.lua"),
        get_attach_action(test_type),
        include_str!("lua_templates/attach.lua"),
    );
    // This is how we actually invoke the action to be tested
    match test_type {
        TestType::Hover | TestType::Completion | TestType::Definition => {
            raw_init = raw_init.replace("LSP_ACTION", &invoke_lsp_action(&test_case.start_type));
        }
        TestType::Diagnostic => {
            // Diagnostics are handled via an autocmd, no need to handle `$/progress`
            raw_init = raw_init.replace("LSP_ACTION", "");
            raw_init.push_str(include_str!("lua_templates/diagnostic_autocmd.lua"));
        }
    };

    let set_cursor_position = test_case.cursor_pos.map_or_else(String::new, |cursor_pos| {
        format!(
            "position = {{ line = {}, character = {} }},",
            cursor_pos.line, cursor_pos.character
        )
    });
    let final_init = raw_init
        .replace("RESULTS_FILE", results_path.to_str().unwrap())
        .replace(
            "EXECUTABLE_PATH",
            test_case.executable_path.to_str().unwrap(),
        )
        .replace("ROOT_PATH", root_path.to_str().unwrap())
        .replace("ERROR_PATH", error_path.to_str().unwrap())
        .replace("LOG_PATH", log_path.to_str().unwrap())
        .replace("FILE_EXTENSION", source_extension)
        .replace("SET_CURSOR_POSITION", &set_cursor_position)
        .replace(
            "PROGRESS_THRESHOLD",
            &progress_threshold(&test_case.start_type),
        )
        .replace(
            "PARENT_PATH",
            test_case
                .get_source_file_path("")
                .unwrap()
                .to_str()
                .unwrap(),
        );

    final_init
}

fn progress_threshold(start_type: &ServerStartType) -> String {
    match start_type {
        ServerStartType::Simple => "0".to_string(),
        ServerStartType::Progress(threshold, _) => threshold.to_string(),
    }
}

fn get_attach_action(test_type: TestType) -> String {
    match test_type {
        TestType::Hover => include_str!("lua_templates/hover_action.lua"),
        TestType::Diagnostic => "\n-- NOTE: No `check_progress_result` function for diagnostics, instead handled by `DiagnosticChanged` autocmd\n",
        TestType::Completion => include_str!("lua_templates/completion_action.lua"),
        TestType::Definition => include_str!("lua_templates/definition_action.lua"),
    }
    .to_string()
}

/// In the simple case, the action is invoked immediately. If a server employs
/// some sort of `$/progress` scheme, then we need to check each time the server
/// claims it's ready, respecting the user-set `progress_threshold`
fn invoke_lsp_action(start_type: &ServerStartType) -> String {
    match start_type {
        // Directly invoke the action. Note we unconditionally end the test after the first try
        ServerStartType::Simple => "check_progress_result()\nvim.cmd('qa!')".to_string(),
        // Hook into `$/progress` messages
        ServerStartType::Progress(_, token_name) => {
            format!(
                r#"vim.lsp.handlers["$/progress"] = function(_, result, _)
                    if client then
                        if result.value.kind == "end" and result.token == "{token_name}" then
                            client.initialized = true
                            check_progress_result()
                        end
                    end
                end"#
            )
        }
    }
}
