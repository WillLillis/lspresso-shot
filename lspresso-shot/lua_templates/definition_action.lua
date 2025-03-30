local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count ~= PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end
    report_log('Issuing definition request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local definition_results = vim.lsp.buf_request_sync(0, "textDocument/definition", {
        textDocument = vim.lsp.util.make_text_document_params(0),
        ---@diagnostic disable-next-line: undefined-global
        SET_CURSOR_POSITION,
    })

    if not definition_results then
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid definition result returned: ' .. vim.inspect(definition_results) .. '\n')
    elseif definition_results and #definition_results > 0 and definition_results[1].result then
        local results_file = io.open('RESULTS_FILE', "w")
        if not results_file then
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            exit() ---@diagnostic disable-line: undefined-global
        end

        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(definition_results[1].result, { escape_slash = true }))
        results_file:close()
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        mark_empty_file() ---@diagnostic disable-line: undefined-global
    end
    exit() ---@diagnostic disable-line: undefined-global
end
