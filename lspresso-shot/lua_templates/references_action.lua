local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count < PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end
    report_log('Issuing reference request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local reference_result = vim.lsp.buf_request_sync(0, 'textDocument/references', {
        textDocument = vim.lsp.util.make_text_document_params(0),
        ---@diagnostic disable: undefined-global
        SET_CURSOR_POSITION,
        SET_CONTEXT
        ---@diagnostic enable: undefined-global
    }, 1000)
    if reference_result and #reference_result >= 1 and reference_result[1].result and #reference_result[1].result >= 1 then
        local results_file = io.open('RESULTS_FILE', 'w')
        if not results_file then
            ---@diagnostic disable-next-line: undefined-global
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            vim.cmd('qa!')
        end
        local refs = reference_result[1].result
        for i, ref in ipairs(refs) do
            ---@diagnostic disable-next-line: undefined-global
            local relative_path = extract_relative_path(ref.uri) ---@diagnostic disable-line: undefined-global
            refs[i].uri = relative_path
        end

        --- NOTE: Does this ever return more than one result? Just use the first for now
        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(refs))
        results_file:close()
        vim.cmd('qa!')
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid reference result returned: ' .. vim.inspect(reference_result) .. '\n') ---@diagnostic disable-line: undefined-global
    end
end
