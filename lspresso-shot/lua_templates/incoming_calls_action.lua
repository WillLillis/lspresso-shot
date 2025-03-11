local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count ~= PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end
    -- Receive  json-encoded `FormattingOptions` from the rust side in `json_opts`
    local json_call_item = [[
CALL_ITEM
    ]]
    local call_item = vim.json.decode(json_call_item)

    report_log('Incoming calls item: ' .. tostring(call_item) .. '\n') ---@diagnostic disable-line: undefined-global
    report_log('Issuing hover request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local incoming_calls_result = vim.lsp.buf_request_sync(0, 'callHierarchy/incomingCalls', {
        item = call_item
    }, 1000)

    if not incoming_calls_result then
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid hover result returned: ' .. vim.inspect(incoming_calls_result) .. '\n')
    elseif incoming_calls_result and #incoming_calls_result >= 1 and incoming_calls_result[1].result then
        local results_file = io.open('RESULTS_FILE', 'w')
        if not results_file then
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            vim.cmd('qa!')
        end
        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(incoming_calls_result[1].result, { escape_slash = true }))
        results_file:close()
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        mark_empty_file() ---@diagnostic disable-line: undefined-global
    end
    vim.cmd('qa!')
end
