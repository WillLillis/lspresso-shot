local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count ~= PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end

    -- Receive  json-encoded `Vec<Position>` from the rust side in `json_positions`
    local json_positions = [[
POSITIONS
    ]]
    local positions = vim.json.decode(json_positions)
    report_log('Positions: ' .. vim.inspect(positions) .. '\n') ---@diagnostic disable-line: undefined-global
    report_log('Issuing selection range request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local selection_range_results = vim.lsp.buf_request_sync(0, "textDocument/selectionRange", {
        textDocument = vim.lsp.util.make_text_document_params(0),
        ---@diagnostic disable-next-line: undefined-global
        positions = positions
    })

    if not selection_range_results then
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid selection range result returned: ' .. vim.inspect(selection_range_results) .. '\n')
    elseif selection_range_results and #selection_range_results > 0 and selection_range_results[1].result then
        local results_file = io.open('RESULTS_FILE', "w")
        if not results_file then
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            vim.cmd('qa!')
        end

        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(selection_range_results[1].result, { escape_slash = true }))
        results_file:close()
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        mark_empty_file() ---@diagnostic disable-line: undefined-global
    end
    vim.cmd('qa!')
end
