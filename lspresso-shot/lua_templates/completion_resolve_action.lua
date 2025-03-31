local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count ~= PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end
    local completion_item_json = [[
    COMPLETION_ITEM
]]
    local completion_item = vim.json.decode(completion_item_json)

    report_log('Completion item: ' .. vim.inspect(completion_item) .. '\n') ---@diagnostic disable-line: undefined-global
    report_log('Issuing completion resolve request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local completion_resolve_result = vim.lsp.buf_request_sync(0, 'completionItem/resolve', {
        -- NOTE: Have to manually destructure here...not sure why
        label = completion_item.label,
        labelDetails = completion_item.labelDetails,
        kind = completion_item.kind,
        tags = completion_item.tags,
        detail = completion_item.detail,
        documentation = completion_item.documentation,
        deprecated = completion_item.deprecated,
        preselect = completion_item.preselect,
        sortText = completion_item.sortText,
        filterText = completion_item.filterText,
        insertText = completion_item.insertText,
        insertTextFormat = completion_item.insertTextFormat,
        insertTextMode = completion_item.insertTextMode,
        textEdit = completion_item.textEdit,
        textEditText = completion_item.textEditText,
        additionalTextEdits = completion_item.additionalTextEdits,
        commitCharacters = completion_item.commitCharacters,
        command = completion_item.command,
        data = completion_item.data,
    })

    if not completion_resolve_result then
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid completion resolve result returned: ' .. vim.inspect(completion_resolve_result) .. '\n')
    elseif completion_resolve_result and #completion_resolve_result >= 1 and completion_resolve_result[1].result then
        local results_file = io.open('RESULTS_FILE', "w")
        if not results_file then
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            exit() ---@diagnostic disable-line: undefined-global
        end
        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(completion_resolve_result[1].result, { escape_slash = true }))
        results_file:close()
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        mark_empty_file() ---@diagnostic disable-line: undefined-global
    end
    exit() ---@diagnostic disable-line: undefined-global
end
