local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count ~= PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end

    -- Receive  json-encoded `CodeLens` from the rust side in `json_opts`
    local json_type_items = [[
ITEMS
    ]]
    local hierarchy_items = vim.json.decode(json_type_items)

    report_log('Type hierarchy items: ' .. tostring(vim.inspect(hierarchy_items)) .. '\n') ---@diagnostic disable-line: undefined-global
    report_log('Issuing Prepare Type Hierarchy  request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local type_hierarchy_result = vim.lsp.buf_request_sync(0, 'textDocument/prepareTypeHierarchy', {
        textDocument = vim.lsp.util.make_text_document_params(0),
        ---@diagnostic disable-next-line: undefined-global
        SET_CURSOR_POSITION,
        items = hierarchy_items, -- TODO: Figure out the actual param name
    })

    if not type_hierarchy_result then
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid type hierarchy result returned: ' .. vim.inspect(type_hierarchy_result) .. '\n')
    elseif type_hierarchy_result and #type_hierarchy_result >= 1 and type_hierarchy_result[1].result then
        local results_file = io.open('RESULTS_FILE', "w")
        if not results_file then
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            exit() ---@diagnostic disable-line: undefined-global
        end
        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(type_hierarchy_result[1].result, { escape_slash = true }))
        results_file:close()
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        mark_empty_file() ---@diagnostic disable-line: undefined-global
    end
    exit() ---@diagnostic disable-line: undefined-global
end
