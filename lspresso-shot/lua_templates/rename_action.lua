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

    -- TODO: This is the logic for parsing a `WorkSpaceEdit` object. We may want
    -- to pull this into a helper eventually for use with other test types
    if rename_result and #rename_result >= 1 and rename_result[1].result then
        local results_file = io.open('RESULTS_FILE', 'w')
        if not results_file then
            ---@diagnostic disable-next-line: undefined-global
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            vim.cmd('qa!')
        end
        if rename_result[1].result.documentChanges then
            -- `WorkSpaceEdit.changes` is `Some`
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
        elseif rename_result[1].result.changes then
            -- `WorkSpaceEdit.doucument_changes` is `Some`
            local result = rename_result[1].result
            -- Create a copy of the HashMap table with relative paths as the keys
            local all_edits = {}
            for uri, text_edits in pairs(rename_result[1].result.changes) do
                ---@diagnostic disable-next-line: undefined-global
                local relative_path = extract_relative_path(uri) ---@diagnostic disable-line: undefined-global
                local uri_edits = {}
                for _, text_edit in ipairs(text_edits) do
                    uri_edits[#uri_edits + 1] = text_edit
                end
                all_edits[relative_path] = uri_edits
            end
            result.changes = all_edits
            --- NOTE: Does this ever return more than one result? Just use the first for now
            ---@diagnostic disable: need-check-nil
            results_file:write(vim.json.encode(result))
            results_file:close()
            vim.cmd('qa!')
            ---@diagnostic enable: need-check-nil
        elseif rename_result[1].result.changeAnnotations then
            -- `WorkSpaceEdit.change_annotations` is `Some`
            local result = rename_result[1].result
            -- Create a copy of the HashMap table with relative paths as the keys
            local all_annotations = {}
            for uri, annotation in pairs(rename_result[1].result.changeAnnotations) do
                ---@diagnostic disable-next-line: undefined-global
                local relative_path = extract_relative_path(uri) ---@diagnostic disable-line: undefined-global
                all_annotations[relative_path] = annotation
            end
            result.changeAnnotations = all_annotations
            --- NOTE: Does this ever return more than one result? Just use the first for now
            ---@diagnostic disable: need-check-nil
            results_file:write(vim.json.encode(result))
            results_file:close()
            vim.cmd('qa!')
            ---@diagnostic enable: need-check-nil
        end
    end
    ---@diagnostic disable-next-line: undefined-global
    report_log('No valid rename result returned: ' .. vim.inspect(rename_result) .. '\n') ---@diagnostic disable-line: undefined-global
end
