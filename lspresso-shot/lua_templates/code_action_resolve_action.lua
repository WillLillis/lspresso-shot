local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count ~= PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end
    local json_params = [[
PARAMS
]]
    local params = vim.json.decode(json_params)
    report_log('Params: ' .. tostring(vim.inspect(params)) .. '\n') ---@diagnostic disable-line: undefined-global

    report_log('Issuing code action resolve request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local code_action_resolve_result = vim.lsp.buf_request_sync(0, 'codeAction/resolve', {
        -- NOTE: Not sure why this has to be destructured manually
        title = params.title,
        kind = params.kind,
        diagnostics = params.diagnostics,
        edit = params.edit,
        command = params.command,
        isPreferred = params.isPreferred,
        disabled = params.disabled,
        data = params.data,
    })

    if not code_action_resolve_result then
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid code action result returned: ' .. vim.inspect(code_action_resolve_result) .. '\n')
    elseif code_action_resolve_result and #code_action_resolve_result >= 1 and code_action_resolve_result[1].result then
        local results_file = io.open('RESULTS_FILE', 'w')
        if not results_file then
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            exit() ---@diagnostic disable-line: undefined-global
        end
        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(code_action_resolve_result[1].result, { escape_slash = true }))
        results_file:close()
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        mark_empty_file() ---@diagnostic disable-line: undefined-global
    end
    exit() ---@diagnostic disable-line: undefined-global
end
