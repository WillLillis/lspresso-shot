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
    report_log('Testing REQUEST_METHOD (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global

    if INVOKE_ACTION then ---@diagnostic disable-line: undefined-global
        report_log('Invoking') ---@diagnostic disable-line: undefined-global
        local results_file = io.open('RESULTS_FILE', 'w')
        if not results_file then
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            exit() ---@diagnostic disable-line: undefined-global
        end

        local start = vim.uv.hrtime()
        INVOKE_FN(params) ---@diagnostic disable-line: undefined-global, exp-in-action
        local elapsed_ns = vim.uv.hrtime() - start
        record_benchmark_result(elapsed_ns) ---@diagnostic disable-line: undefined-global
        local lines = vim.api.nvim_buf_get_lines(0, 0, -1, true)
        local final_state = table.concat(lines, "\\n")
        ---@diagnostic disable: need-check-nil
        results_file:write('"' .. final_state .. '"')
        results_file:close()
        ---@diagnostic enable: need-check-nil
    else
        report_log('Requesting') ---@diagnostic disable-line: undefined-global
        local start = vim.uv.hrtime()
        local resp = vim.lsp.buf_request_sync(0, 'REQUEST_METHOD', params)
        local elapsed_ns = vim.uv.hrtime() - start
        record_benchmark_result(elapsed_ns) ---@diagnostic disable-line: undefined-global
        if not resp then
            ---@diagnostic disable-next-line: undefined-global
            report_log('No valid formatting result returned: ' .. vim.inspect(resp) .. '\n')
        elseif resp and #resp >= 1 and resp[1].result then
            local results_file = io.open('RESULTS_FILE', 'w')
            if not results_file then
                report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
                exit() ---@diagnostic disable-line: undefined-global
            end
            ---@diagnostic disable: need-check-nil
            results_file:write(vim.json.encode(resp[1].result, { escape_slash = true }))
            results_file:close()
            ---@diagnostic enable: need-check-nil
        else
            ---@diagnostic disable-next-line: undefined-global
            mark_empty_file() ---@diagnostic disable-line: undefined-global
        end
    end
    exit() ---@diagnostic disable-line: undefined-global
end
