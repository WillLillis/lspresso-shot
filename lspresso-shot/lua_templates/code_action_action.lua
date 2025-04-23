local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count ~= PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end
    local json_context = [[
CONTEXT
]]
    local json_range = [[
RANGE
]]
    local context = vim.json.decode(json_context)
    local range = vim.json.decode(json_range)
    report_log('Params: ' .. tostring(vim.inspect(context)) .. '\n') ---@diagnostic disable-line: undefined-global
    report_log('Range: ' .. tostring(vim.inspect(range)) .. '\n') ---@diagnostic disable-line: undefined-global

    report_log('Issuing code action request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local hover_result = vim.lsp.buf_request_sync(0, 'textDocument/codeAction', {
        textDocument = vim.lsp.util.make_text_document_params(0),
        ---@diagnostic disable: undefined-global
        context = context,
        range = range,
        ---@diagnostic enable: undefined-global
    })

    if not hover_result then
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid code action result returned: ' .. vim.inspect(hover_result) .. '\n')
    elseif hover_result and #hover_result >= 1 and hover_result[1].result then
        local results_file = io.open('RESULTS_FILE', 'w')
        if not results_file then
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            exit() ---@diagnostic disable-line: undefined-global
        end
        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(hover_result[1].result, { escape_slash = true }))
        results_file:close()
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        mark_empty_file() ---@diagnostic disable-line: undefined-global
    end
    exit() ---@diagnostic disable-line: undefined-global
end
