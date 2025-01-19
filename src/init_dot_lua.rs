use std::path::Path;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum InitType {
    Hover,
    Diagnostic,
}

/// Construct the contents of an init.lua file to test an lsp request corresponding
/// to `init_type` using the given parameters
pub fn get_init_dot_lua(
    init_type: InitType,
    root_path: &Path,
    results_path: &Path,
    error_path: &Path,
    executable_path: &Path,
    source_extension: &str,
) -> String {
    let mut raw_init = format!("{ERROR_REPORT}{FILETYPE_ADD}{FILETYPE_AUTOCMD}");
    raw_init.push_str(match init_type {
        InitType::Hover => HOVER_INIT_DOT_LUA,
        InitType::Diagnostic => DIAGNOSTIC_INIT_DOT_LUA,
    });
    raw_init
        .replace("RESULTS_FILE", results_path.to_str().unwrap())
        .replace("EXECUTABLE_PATH", executable_path.to_str().unwrap())
        .replace("ROOT_PATH", root_path.to_str().unwrap())
        .replace("ERROR_PATH", error_path.to_str().unwrap())
        .replace("FILE_EXTENSION", source_extension)
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
            return
        end
        vim.lsp.start {
            name = 'lspresso_shot',
            cmd = { 'EXECUTABLE_PATH' },
            root_dir = 'ROOT_PATH/src',
            settings = {},
        }
    end,
})
";

/// Invoke a 'textDocument/hover' request when the LSP starts, gather the results
/// and write them to a file in TOML format
const HOVER_INIT_DOT_LUA: &str = r#"
vim.api.nvim_create_autocmd('LspAttach', {
    callback = function(_)
        local pos = { CURSOR_LINE, CURSOR_COLUMN }
        vim.api.nvim_win_set_cursor(0, pos)
        local hover_result = vim.lsp.buf_request_sync(0, 'textDocument/hover', {
            textDocument = vim.lsp.util.make_text_document_params(0),
            position = { line = pos[1] - 1, character = pos[2] },
        }, 1000)
        if hover_result then
            -- Write the results in a TOML format for easy deserialization
            local file = io.open('RESULTS_FILE', 'w')
            if file then
                file:write('kind = "' .. tostring(hover_result[1].result.contents.kind .. '"\n'))
                file:write('value = """\n' .. tostring(hover_result[1].result.contents.value .. '\n"""'))
                file:close()
            end
        else 
            report_error('No hover result returned')
        end
        vim.cmd('qa!')
    end,
})
"#;

/// Invoke a 'textDocument/publishDiagnostics' request when the LSP starts, gather the results
/// and write them to a file in TOML format
const DIAGNOSTIC_INIT_DOT_LUA: &str = r#"
vim.api.nvim_create_autocmd('DiagnosticChanged', {
    callback = function(_)
        local diagnostics_result = vim.diagnostic.get(0, {})
        if diagnostics_result then
            local file = io.open('RESULTS_FILE', 'w')
            if file then
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
            end
        else
            report_error('No diagnostic result returned')
        end
        vim.cmd('qa!')
    end,
})
"#;
