local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    report_log('Issuing hover request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local hover_result = vim.lsp.buf_request_sync(0, 'textDocument/hover', {
        textDocument = vim.lsp.util.make_text_document_params(0),
        ---@diagnostic disable-next-line: undefined-global
        SET_CURSOR_POSITION
    }, 1000)
    local results_file = io.open('RESULTS_FILE', 'w')
    if not results_file then
        report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
        vim.cmd('qa!')
    end
    if hover_result and #hover_result >= 1 and hover_result[1].result then
        local cleaned = hover_result[1]
        cleaned.result.contents.value = string.gsub(cleaned.result.contents.value, "\\\\", "\\") -- HACK: find a better way?
        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(cleaned.result))
        results_file:close()
        ---@diagnostic enable: need-check-nil
        vim.cmd('qa!')
    else
        ---@diagnostic disable: undefined-global
        report_log('No hover result returned (Attempt ' ..
            tostring(progress_count) .. '):\n ' .. vim.inspect(hover_result) .. '\n\n')
        ---@diagnostic enable: undefined-global
    end
    progress_count = progress_count + 1
end
