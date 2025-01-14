use std::path::Path;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum InitType {
    Hover,
    Diagnostic,
}

/// Construct the contents of an init.lua file to test an lsp request corresponding
/// to `init_type` using the given parameters
pub(crate) fn get_init_dot_lua(
    init_type: InitType,
    root_path: &Path,
    results_path: &Path,
    executable_path: &Path,
    source_extension: &str,
) -> String {
    let mut raw_init = format!("{QUIT_ON_INVALID}{FILETYPE_ADD}{FILETYPE_AUTOCMD}");
    raw_init.push_str(match init_type {
        InitType::Hover => HOVER_INIT_DOT_LUA,
        InitType::Diagnostic => DIAGNOSTIC_INIT_DOT_LUA,
    });
    raw_init
        .replace("RESULTS_FILE", results_path.to_str().unwrap())
        .replace("EXECUTABLE_PATH", executable_path.to_str().unwrap())
        .replace("ROOT_PATH", root_path.to_str().unwrap())
        .replace("FILE_EXTENSION", source_extension)
}

// TODO: In case of an error/ unexpected conditions on the neovim side, write to
// a common error file so we can pick up and display errors from rust

/// If we pass an invalid path to the Neovim instance, e.g. a directory, the test
/// will hang indefinitely. This causes a clean exit instead
const QUIT_ON_INVALID: &str = r#"
vim.api.nvim_create_autocmd("BufEnter", {
    pattern = "*",
    callback = function(ev)
        if vim.bo[ev.buf].buftype == '' then
            vim.cmd('qa!')
        end
    end,
})
"#;

/// Add our custom extension as a filetype to use for our LSP to match against
const FILETYPE_ADD: &str = r#"
vim.filetype.add({
    extension = {
        FILE_EXTENSION = 'lspresso_shot',
    },
})
"#;

/// Start the LSP on '*.FILE_EXTENSION' filetypes
const FILETYPE_AUTOCMD: &str = r#"
vim.api.nvim_create_autocmd('FileType', {
    pattern = 'lspresso_shot',
    callback = function(ev)
        if vim.bo[ev.buf].buftype == 'nofile' then
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
"#;

/// Invoke a 'textDocument/hover` request when the LSP starts, gather the results
/// and write them to a file in TOML format
const HOVER_INIT_DOT_LUA: &str = r#"
vim.api.nvim_create_autocmd('LspAttach', {
    callback = function(_)
        local file = io.open('RESULTS_FILE', 'w')
        if file then
            local pos = { CURSOR_LINE, CURSOR_COLUMN }
            vim.api.nvim_win_set_cursor(0, pos)
            local hover_result = vim.lsp.buf_request_sync(0, 'textDocument/hover', {
                textDocument = vim.lsp.util.make_text_document_params(0),
                position = { line = pos[1] - 1, character = pos[2] },
            }, 1000)
            if hover_result then
                -- Write the results in a TOML format for easy deserialization
                file:write('kind = "' .. tostring(hover_result[1].result.contents.kind .. '"\n'))
                file:write('value = """\n' .. tostring(hover_result[1].result.contents.value .. '\n"""'))
                file:flush()
                file:close()
            end
        end
        vim.cmd('qa!')
    end,
})
"#;

// TODO: Rework this...
const DIAGNOSTIC_INIT_DOT_LUA: &str = r#"
vim.api.nvim_create_autocmd('DiagnosticChanged', {
    callback = function(_)
        local file = io.open('RESULTS_FILE', 'w')
        if file then
            local diagnostics_result = vim.diagnostic.get(0, {})
            -- TODO: Figure out a TOML format (which pieces of info do we care about?)
            file:write(vim.inspect(diagnostics_result))
            file:close()
        end
        vim.cmd('qa!')
    end,
})
"#;
