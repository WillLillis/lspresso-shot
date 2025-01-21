vim.api.nvim_create_autocmd('LspAttach', {
    callback = function(_)
        local completion_results = vim.lsp.buf_request_sync(0, "textDocument/completion", {
            textDocument = vim.lsp.util.make_text_document_params(0),
            position = { line = 20 - 1, character = 10 },
        }, 1000)
        local file = io.open('/tmp/lspresso-shot/16502638915857989137/results.toml', "w")
        if completion_results and file then
            for _, result in pairs(completion_results) do
                if result.result and result.result.items then
                    for _, item in ipairs(result.result.items) do
                        file:write('[[completions]]\n')
                        local label = string.gsub(item.label, "\\", "\\\\") -- serde fails to parse, interpreting slashes as escape sequences
                        file:write('label = "' .. label .. '"\n')
                        file:write('kind = "' .. tostring(item.kind) .. '"\n')
                        file:write('documentation_kind = "' .. item.documentation.kind .. '"\n')
                        local raw_value = tostring(item.documentation.value)
                        local value = string.gsub(raw_value, "\\", "\\\\") -- serde fails to parse, interpreting slashes as escape sequences
                        file:write('documentation_value = """\n' .. value .. '\n"""\n\n')
                    end
                end
            end
            file:close()
        else
            report_error('No completion result returned')
        end
        vim.cmd('qa!')
    end,
})
