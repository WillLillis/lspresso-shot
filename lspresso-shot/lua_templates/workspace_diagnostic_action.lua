local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count ~= PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end
    local identifier_json = [[
    IDENTIFIER
]]
    local previous_result_ids_json = [[
    PREVIOUS_RESULT_ID
]]
    local identifier = vim.json.decode(identifier_json)
    local previous_result_ids = vim.json.decode(previous_result_ids_json)
    report_log('Identifier: ' .. vim.inspect(identifier) .. '\n') ---@diagnostic disable-line: undefined-global
    report_log('Previous Result ID: ' .. vim.inspect(previous_result_id) .. '\n') ---@diagnostic disable-line: undefined-global
    report_log('Issuing diagnostic request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local diagnostic_result = vim.lsp.buf_request_sync(0, 'workspace/diagnostic', {
        textDocument = vim.lsp.util.make_text_document_params(0),
        identifier = identifier,
        previousResultIds = previous_result_ids,
    })

    if not diagnostic_result then
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid workspace diagnostic result returned: ' .. vim.inspect(diagnostic_result) .. '\n')
    elseif diagnostic_result and #diagnostic_result >= 1 and diagnostic_result[1].result then
        local results_file = io.open('RESULTS_FILE', 'w')
        if not results_file then
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            exit() ---@diagnostic disable-line: undefined-global
        end
        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(diagnostic_result[1].result, { escape_slash = true }))
        results_file:close()
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        mark_empty_file() ---@diagnostic disable-line: undefined-global
    end
    exit() ---@diagnostic disable-line: undefined-global
end
