local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count ~= PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end
    local params = {}
    ---@diagnostic disable-next-line: undefined-global, exp-in-action
    PARAM_ASSIGN

    report_log('Issuing semantic tokens full request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local semantic_tokens_full_result = vim.lsp.buf_request_sync(0, 'textDocument/semanticTokens/full', params)

    local result_id = nil
    if not semantic_tokens_full_result then
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid semantic tokens full result returned: ' .. vim.inspect(semantic_tokens_full_result) .. '\n') ---@diagnostic disable-line: undefined-global
        exit() ---@diagnostic disable-line: undefined-global
    elseif semantic_tokens_full_result and #semantic_tokens_full_result >= 1 and semantic_tokens_full_result[1].result then
        result_id = semantic_tokens_full_result[1].result.resultId
    else
        ---@diagnostic disable-next-line: undefined-global
        report_log('Empty semantic tokens full result returned: ' .. vim.inspect(semantic_tokens_full_result) .. '\n')
        exit() ---@diagnostic disable-line: undefined-global
    end

    if not result_id then
        report_error('nil resultId returned') ---@diagnostic disable-line: undefined-global
        exit() ---@diagnostic disable-line: undefined-global
    end

    report_log('Issuing semantic tokens full delta request\n') ---@diagnostic disable-line: undefined-global
    local semantic_tokens_full_delta_result = vim.lsp.buf_request_sync(0, 'textDocument/semanticTokens/full/delta', {
        textDocument = vim.lsp.util.make_text_document_params(0),
        previousResultId = result_id,
    })
    if not semantic_tokens_full_delta_result then
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid semantic tokens full delta result returned: ' ..
            vim.inspect(semantic_tokens_full_delta_result) .. '\n') ---@diagnostic disable-line: undefined-global
    elseif semantic_tokens_full_delta_result and #semantic_tokens_full_delta_result >= 1 and semantic_tokens_full_delta_result[1].result then
        local results_file = io.open('RESULTS_FILE', "w")
        if not results_file then
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            exit() ---@diagnostic disable-line: undefined-global
        end

        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(semantic_tokens_full_delta_result[1].result, { escape_slash = true }))
        results_file:close()
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        report_log('Empty semantic tokens full delta result returned: ' ..
            vim.inspect(semantic_tokens_full_delta_result) .. '\n')
        mark_empty_file() ---@diagnostic disable-line: undefined-global
    end
    exit() ---@diagnostic disable-line: undefined-global
end
