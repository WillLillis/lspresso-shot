---@param msg string
---@diagnostic disable-next-line: unused-local, unused-function
local function report_error(msg)
    local error_file = io.open('/tmp/lspresso-shot/10334665970033721168/error.txt', 'a')
    if error_file then
        error_file:write(msg)
        error_file:close()
    end
end

---@param msg string
---@diagnostic disable-next-line: unused-local, unused-function
local function report_log(msg)
    local log_file = io.open('/tmp/lspresso-shot/10334665970033721168/log.txt', 'a')
    if log_file then
        log_file:write(msg)
        log_file:close()
    end
end

--- Extracts the relative path from a file:// URI
---@param uri string
---@return string
---@diagnostic disable-next-line: unused-local, unused-function
local function extract_relative_path(uri)
    if not string.sub(uri, 1, 7) == 'file://' then
        report_error('URI is not a file:// URI')
    end
    local path = vim.uri_to_fname(uri)
    return string.sub(path,
        string.len('/tmp/lspresso-shot/10334665970033721168/src/') + 1,
        string.len(path))
end
local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    local comps = vim.lsp.buf_request_sync(0, 'textDocument/completion', {
        textDocument = vim.lsp.util.make_text_document_params(0),
        position = { line = 1, character = 10 },
    })
    if not comps then
        return
    end
    if #comps > 1 then
        print('wtf man')
    end
    for _, comp in ipairs(comps) do
        if comp.result then
            for _, item in ipairs(comp.result.items) do
                if item.kind and item.kind ~= 14 and item.kind ~= 15 and item.kind ~= 22 then
                    if item.textEdit and string.find(item.textEdit.newText, "println!") then
                        print(vim.inspect(item))
                        vim.fn.complete_add('your mom')
                    end
                end
            end
        end
    end
end

vim.filetype.add({
    extension = {
        rs = 'lspresso_shot',
    },
})

vim.api.nvim_create_autocmd('FileType', {
    pattern = 'lspresso_shot',
    callback = function(ev)
        if vim.bo[ev.buf].buftype == 'nofile' then
            report_error('Invalid buffer type opened') ---@diagnostic disable-line: undefined-global
            vim.cmd('qa!')
        end
        vim.lsp.start {
            name = 'lspresso_shot',
            cmd = { 'rust-analyzer' },
            root_dir = '/tmp/lspresso-shot/10334665970033721168/src',
            settings = {},
            on_attach = function(client, _) ---@diagnostic disable-line: unused-local
                ---@diagnostic disable-next-line: undefined-global, exp-in-action

                vim.lsp.handlers["$/progress"] = function(_, result, _)
                    if client then
                        if result.value.kind == "end" and result.token == "rustAnalyzer/Indexing" then
                            client.initialized = true
                            check_progress_result()
                        end
                    end
                end
            end,
        }
    end,
})
