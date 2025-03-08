local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count < PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end
    report_log('Issuing reference request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local reference_result = vim.lsp.buf_request_sync(0, 'textDocument/references', {
        textDocument = vim.lsp.util.make_text_document_params(0),
        ---@diagnostic disable: undefined-global
        SET_CURSOR_POSITION,
        SET_CONTEXT
        ---@diagnostic enable: undefined-global
    }, 1000)
    if not reference_result then
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid reference result returned: ' .. vim.inspect(reference_result) .. '\n') ---@diagnostic disable-line: undefined-global
    elseif reference_result and #reference_result >= 1 and reference_result[1].result then
        local results_file = io.open('RESULTS_FILE', 'w')
        if not results_file then
            ---@diagnostic disable-next-line: undefined-global
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            vim.cmd('qa!')
        end

        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(reference_result[1].result, { escape_slash = true }))
        results_file:close()
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        mark_empty_file() ---@diagnostic disable-line: undefined-global
    end
    vim.cmd('qa!')
end
