local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count ~= PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end
    report_log('Issuing semantic tokens full request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local semantic_tokens_full_result = vim.lsp.buf_request_sync(0, 'textDocument/semanticTokens/full', {
        textDocument = vim.lsp.util.make_text_document_params(0),
    }, 1000)

    if not semantic_tokens_full_result then
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid semantic tokens full result returned: ' .. vim.inspect(semantic_tokens_full_result) .. '\n') ---@diagnostic disable-line: undefined-global
    elseif semantic_tokens_full_result and #semantic_tokens_full_result >= 1 and semantic_tokens_full_result[1].result then
        local results_file = io.open('RESULTS_FILE', 'w')
        if not results_file then
            ---@diagnostic disable-next-line: undefined-global
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            vim.cmd('qa!')
        end
        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(semantic_tokens_full_result[1].result, { escape_slash = true }))
        results_file:close()
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        mark_empty_file() ---@diagnostic disable-line: undefined-global
    end
    vim.cmd('qa!')
end
