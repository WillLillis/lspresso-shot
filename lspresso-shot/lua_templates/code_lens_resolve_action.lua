local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count ~= PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end

    -- Receive  json-encoded `CodeLens` from the rust side in `json_opts`
    local json_code_lens = [[
CODE_LENS
    ]]
    local code_lens = vim.json.decode(json_code_lens)

    report_log('Code Lens: ' .. tostring(vim.inspect(code_lens)) .. '\n') ---@diagnostic disable-line: undefined-global
    report_log('Issuing code lens resolve request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local code_lens_result = vim.lsp.buf_request_sync(0, 'codeLens/resolve', {
        -- NOTE: Not sure why, but we have to manually destructure the data here
        range = code_lens.range,
        command = code_lens.command,
        data = code_lens.data,
    })

    if not code_lens_result then
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid code lens resolve result returned: ' .. vim.inspect(code_lens_result) .. '\n')
    elseif code_lens_result and #code_lens_result >= 1 and code_lens_result[1].result then
        local results_file = io.open('RESULTS_FILE', "w")
        if not results_file then
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            exit() ---@diagnostic disable-line: undefined-global
        end
        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(code_lens_result[1].result, { escape_slash = true }))
        results_file:close()
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        mark_empty_file() ---@diagnostic disable-line: undefined-global
    end
    exit() ---@diagnostic disable-line: undefined-global
end
