use std::path::Path;

use crate::types::{ServerStartType, TestCase, TestType};

/// Construct the contents of an init.lua file to test an lsp request corresponding
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
    // Start out with an error reporting utility, adding the relevant filetype, and the relevant
    // `check_progress_result` function to invoke our request at the appropriate time
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
            // Diagnostics are handled via an autocommand, no need to handle $/progress
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
            "PARENT_PATH",
            test_case
                .get_source_file_path("")
                .unwrap()
                .to_str()
                .unwrap(),
        );
    final_init
}

fn get_attach_action(test_type: TestType) -> String {
    match test_type {
        TestType::Hover => include_str!("lua_templates/hover_action.lua"),
        // Diagnostic results are gathered via the `DiagnosticChanged` autocmd
        TestType::Diagnostic => "",
        TestType::Completion => COMPLETION_ACTION, // TODO: Fix
        TestType::Definition => include_str!("lua_templates/definition_action.lua"),
    }
    .to_string()
}

/// In the simple case, the action is invoked immediately. If a server employs
/// some sort of `$/progress` scheme, then we need to wait until it's completed
/// before issuing a request
fn invoke_lsp_action(start_type: &ServerStartType) -> String {
    match start_type {
        // Directly invoke the action
        ServerStartType::Simple => "check_progress_result()".to_string(),
        // Setup polling to check if the server is ready
        ServerStartType::Progress(token_name) => {
            format!(
                r#"
vim.lsp.handlers["$/progress"] = function(_, result, _)
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

// TODO: Rework this once we have completions figured out
/// Invoke a 'textDocument/publishDiagnostics' request, gather the results, and
/// write them to a file in TOML format
const COMPLETION_ACTION: &str = r#"
local progress_count = 0 -- track how many times we've tried for the logs

local function check_progress_result()
    local completion_results = vim.lsp.buf_request_sync(0, "textDocument/completion", {
        textDocument = vim.lsp.util.make_text_document_params(0),
        SET_CURSOR_POSITION
    }, 1000)
    local file = io.open('RESULTS_FILE', "w")
    if completion_results and #completion_results > 1 and completion_results[1].result and completion_results[1].result.items and file then
        local t = { }
        for _, result in pairs(completion_results) do
            if result.result and result.result.items then
                for _, item in ipairs(result.result.items) do
                    t[#t+1] = '[[completions]]\n'
                    local label = string.gsub(item.label, "\\", "\\\\") -- serde fails to parse, interpreting slashes as escape sequences
                    t[#t+1] = 'label = "' .. label .. '"'
                    t[#t+1] = 'kind = "' .. tostring(item.kind) .. '"'
                    t[#t+1] = 'documentation_kind = "' .. item.documentation.kind .. '"'
                    local raw_value = tostring(item.documentation.value)
                    local value = string.gsub(raw_value, "\\", "\\\\") -- serde fails to parse, interpreting slashes as escape sequences
                    t[#t+1] = 'documentation_value = """\n' .. value .. '\n"""\n'
                end
            end
        end
        local completions = table.concat(t, '\n')
        file:write(completions)
        file:close()
        vim.cmd('qa!')
    else
        report_log('No completion result returned (Attempt ' .. tostring(progress_count) .. ')\n')
    end
    progress_count = progress_count + 1
end
"#;
