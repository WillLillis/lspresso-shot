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

    report_log('Params: ' .. tostring(vim.inspect(params)) .. '\n') ---@diagnostic disable-line: undefined-global
    report_log('Issuing REQUEST_METHOD request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local start = vim.uv.hrtime()
    local req_result = vim.lsp.buf_request_sync(0, 'REQUEST_METHOD', params)
    local elapsed_ns = vim.uv.hrtime() - start
    record_benchmark_result(elapsed_ns) ---@diagnostic disable-line: undefined-global

    if not req_result then
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid REQUEST_METHOD result returned: ' .. vim.inspect(req_result) .. '\n')
    elseif req_result and #req_result >= 1 and req_result[1].result then
        local results_file = io.open('RESULTS_FILE', 'w')
        if not results_file then
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            exit() ---@diagnostic disable-line: undefined-global
        end
        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(req_result[1].result, { escape_slash = true }))
        results_file:close()
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        mark_empty_file() ---@diagnostic disable-line: undefined-global
    end
    exit() ---@diagnostic disable-line: undefined-global
end
