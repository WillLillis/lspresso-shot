local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count < PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end
    report_log('Issuing rename request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local rename_result = vim.lsp.buf_request_sync(0, 'textDocument/rename', {
        textDocument = vim.lsp.util.make_text_document_params(0),
        ---@diagnostic disable: undefined-global
        SET_CURSOR_POSITION,
        NEW_NAME
        ---@diagnostic enable: undefined-global
    }, 1000)

    if not rename_result then
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid rename result returned: ' .. vim.inspect(rename_result) .. '\n') ---@diagnostic disable-line: undefined-global
    elseif rename_result and #rename_result >= 1 and rename_result[1].result then
        local results_file = io.open('RESULTS_FILE', 'w')
        if not results_file then
            ---@diagnostic disable-next-line: undefined-global
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            vim.cmd('qa!')
        end
        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(rename_result[1].result))
        results_file:close()
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        mark_empty_file() ---@diagnostic disable-line: undefined-global
    end
    vim.cmd('qa!')
end
