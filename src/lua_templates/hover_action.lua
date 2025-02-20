local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count < PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end
    report_log('Issuing hover request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local hover_result = vim.lsp.buf_request_sync(0, 'textDocument/hover', {
        textDocument = vim.lsp.util.make_text_document_params(0),
        ---@diagnostic disable-next-line: undefined-global
        SET_CURSOR_POSITION,
    }, 1000)
    if hover_result and #hover_result >= 1 and hover_result[1].result and
        hover_result[1].result.contents and hover_result[1].result.contents.value then
        local results_file = io.open('RESULTS_FILE', 'w')
        if not results_file then
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            vim.cmd('qa!')
        end
        local cleaned = hover_result[1]
        cleaned.result.contents.value = string.gsub(cleaned.result.contents.value, "\\\\", "\\") -- HACK: find a better way?
        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(cleaned.result))
        results_file:close()
        vim.cmd('qa!')
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid hover result returned: ' .. vim.inspect(hover_result) .. '\n')
    end
end
