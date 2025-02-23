local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count < PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end
    report_log('Issuing rename request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local rename_result = vim.lsp.buf_request_sync(0, 'textDocument/rename', {
        textDocument = vim.lsp.util.make_text_document_params(0),
        ---@diagnostic disable: undefined-global
        SET_CURSOR_POSITION,
        NEW_NAME
        ---@diagnostic enable: undefined-global
    }, 1000)
    -- TODO: Handle the `changes` edit type. It's stored as a HashMap<Uri, Vec<TextEdit>>
    -- on the Rust side, so we'll have to clean up its uri too
    if rename_result and #rename_result >= 1 and rename_result[1].result and rename_result[1].result.documentChanges then
        local results_file = io.open('RESULTS_FILE', 'w')
        if not results_file then
            ---@diagnostic disable-next-line: undefined-global
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            vim.cmd('qa!')
        end
        local doc_changes = rename_result[1].result.documentChanges
        for i, edit in ipairs(doc_changes) do
            ---@diagnostic disable-next-line: undefined-global
            local relative_path = extract_relative_path(edit.textDocument.uri) ---@diagnostic disable-line: undefined-global
            doc_changes[i].textDocument.uri = relative_path
        end

        --- NOTE: Does this ever return more than one result? Just use the first for now
        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(rename_result[1].result))
        results_file:close()
        vim.cmd('qa!')
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid rename result returned: ' .. vim.inspect(rename_result) .. '\n') ---@diagnostic disable-line: undefined-global
    end
end
