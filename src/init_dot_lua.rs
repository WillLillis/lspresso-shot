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
        "{ERROR_REPORT}{LOG_REPORT}{FILETYPE_ADD}{}",
        get_attach_action(test_type)
    );
    // This is how we actually invoke the action to be tested
    let action = match test_type {
        TestType::Hover | TestType::Completion | TestType::Definition => FILETYPE_AUTOCMD.replace(
            "LSP_ACTION",
            &invoke_lsp_action(&test_case.start_type).to_string(),
        ),
        TestType::Diagnostic => {
            // Diagnostics are handled via an autocommand, no need to handle $/progress
            let mut init = FILETYPE_AUTOCMD.replace("LSP_ACTION", "");
            init.push_str(DIAGNOSTIC_AUTOCMD);
            init
        }
    };
    raw_init.push_str(&action);

    let set_cursor_position = if let Some(cursor_pos) = test_case.cursor_pos {
        format!(
            "position = {{ line = {}, character = {} }},",
            cursor_pos.line, cursor_pos.column
        )
    } else {
        String::new()
    };
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
        TestType::Hover => HOVER_ACTION,
        // Diagnostic results are gathered via the `DiagnosticChanged` autocmd
        TestType::Diagnostic => "",
        TestType::Completion => COMPLETION_ACTION,
        TestType::Definition => DEFINITION_ACTION,
    }
    .to_string()
}

/// Helper to write any errors that occurred to `ERROR_PATH`
const ERROR_REPORT: &str = "
local function report_error(msg)
    local error_file = io.open('ERROR_PATH', 'a')
    if error_file then
        error_file:write(msg)
        error_file:close()
    end
end
";

// TODO: Make better use of the log file for sake of debugging...

/// Helper to write any errors that occurred to `LOG_PATH`
const LOG_REPORT: &str = "
local function report_log(msg)
    local log_file = io.open('LOG_PATH', 'a')
    if log_file then
        log_file:write(msg)
        log_file:close()
    end
end
";

/// Add our custom extension as a filetype to use for our LSP to match against
const FILETYPE_ADD: &str = "
vim.filetype.add({
    extension = {
        FILE_EXTENSION = 'lspresso_shot',
    },
})
";

/// Start the LSP on *.`FILE_EXTENSION` filetypes
const FILETYPE_AUTOCMD: &str = "
vim.api.nvim_create_autocmd('FileType', {
    pattern = 'lspresso_shot',
    callback = function(ev)
        if vim.bo[ev.buf].buftype == 'nofile' then
            report_error('Invalid buffer type opened')
            vim.cmd('qa!')
        end
        vim.lsp.start {
            name = 'lspresso_shot',
            cmd = { 'EXECUTABLE_PATH' },
            root_dir = 'ROOT_PATH/src',
            settings = {},
            on_attach = function(client, _)
                LSP_ACTION
            end,
        }
    end,
})
";

/// In the simple case, the action is invoked immediately. If a server employs
/// some sort of `$/progress` scheme, then we need to wait until it's completed
/// before issuing a request
fn invoke_lsp_action(start_type: &ServerStartType) -> String {
    match start_type {
        ServerStartType::Simple => "check_progress_result()".to_string(),
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

/// Invoke a 'textDocument/hover' request, gather the results, and write them to
/// a file in TOML format
const HOVER_ACTION: &str = r#"
local progress_count = 0 -- track how many times we've tried for the logs

local function check_progress_result()
    local hover_result = vim.lsp.buf_request_sync(0, 'textDocument/hover', {
        textDocument = vim.lsp.util.make_text_document_params(0),
        SET_CURSOR_POSITION
    }, 1000)
    -- Write the results in a TOML format for easy deserialization
    local file = io.open('RESULTS_FILE', 'w')
    if hover_result and #hover_result >= 1 and hover_result[1].result and file then
        file:write('kind = "' .. tostring(hover_result[1].result.contents.kind .. '"\n'))
        local value = string.gsub(hover_result[1].result.contents.value, "\\", "\\\\") -- escape invisibles
        file:write('value = """\n' .. value .. '\n"""')
        file:close()
        vim.cmd('qa!')
    else
        report_log('No hover result returned (Attempt ' .. tostring(progress_count) .. ')\n')
    end
    progress_count = progress_count + 1
end
"#;

/// Invoke a 'textDocument/publishDiagnostics' request, gather the results, and
/// write them to a file in TOML format
const DIAGNOSTIC_AUTOCMD: &str = r#"
local progress_count = 0 -- track how many times we've tried for the logs

vim.api.nvim_create_autocmd('DiagnosticChanged', {
    callback = function(_)
        local diagnostics_result = vim.diagnostic.get(0, {})
        local file = io.open('RESULTS_FILE', 'w')
        if diagnostics_result and #diagnostics_result >= 1 and file then
            for _, diagnostic in pairs(diagnostics_result) do
                file:write('[[diagnostics]]\n' )
                file:write('start_line = ' .. tostring(diagnostic.lnum) .. '\n')
                file:write('start_character = ' .. tostring(diagnostic.col) .. '\n')
                file:write('message = """\n' .. diagnostic.message .. '\n"""\n')
                if diagnostic.end_lnum then
                    file:write('end_line = ' .. tostring(diagnostic.end_lnum) .. '\n')
                end
                if diagnostic.end_col then
                    file:write('end_character = ' .. tostring(diagnostic.col) .. '\n')
                end
                if diagnostic.severity then
                    file:write('severity = "' .. tostring(diagnostic.severity) .. '"\n')
                end
                file:write('\n')
            end
            file:close()
            vim.cmd('qa!')
        else
            report_log('No diagnostic result returned (Attempt ' .. tostring(progress_count) .. ')\n')
        end
        progress_count = progress_count + 1
    end,
})
"#;

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

// TODO: Need to handle both cases, one where a simple definition result is returned,
// and the other where a list of definition results is returned
/// Invoke a 'textDocument/publishDiagnostics' request, gather the results, and
/// write them to a file in TOML format
const DEFINITION_ACTION: &str = r#"
local progress_count = 0 -- track how many times we've tried for the logs

local function check_progress_result()
    local definition_results = vim.lsp.buf_request_sync(0, "textDocument/definition", {
        textDocument = vim.lsp.util.make_text_document_params(0),
        SET_CURSOR_POSITION
    }, 1000)
    local file = io.open('RESULTS_FILE', "w")
    if definition_results and #definition_results >= 1 and definition_results[1].result and #definition_results[1].result >= 1 and file then
        local result = definition_results[1].result[1]
        -- local range = definition_results[1].result.range
        local range = result.targetRange
        -- local path = string.gsub(definition_results[1].result.uri, 'file://', '')
        local path = nil
        if result.targetUri then
            path = string.gsub(result.targetUri, 'file://', '')
        elseif result.uri  then
            path = string.gsub(result.uri, 'file://', '')
        else
            report_error('No path found in definition result\n')
            progress_count = progress_count + 1
            return
        end

        local relative_path = string.sub(path, string.len('PARENT_PATH') + 1, string.len(path))
        file:write('path = "' .. relative_path .. '"\n\n')
        file:write('start_line = ' .. tostring(range.start.line) .. '\n')
        file:write('start_column = ' .. tostring(range.start.character) .. '\n')
        if range['end'] then
            file:write('end_line = ' .. tostring(range['end'].line) .. '\n')
            file:write('end_column = ' .. tostring(range['end'].character) .. '\n')
        end
        file:close()
        vim.cmd('qa!')
    else
        report_log('No definition result returned (Attempt ' .. tostring(progress_count) .. ')\n')
    end
    progress_count = progress_count + 1
end
"#;
