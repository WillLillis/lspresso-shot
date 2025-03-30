local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count ~= PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end

    -- Receive  json-encoded `DocumentLink` from the rust side in `json_link`
    local json_link = [[
DOC_LINK
    ]]
    local doc_link = vim.json.decode(json_link)

    report_log('Document link: ' .. tostring(vim.inspect(doc_link)) .. '\n') ---@diagnostic disable-line: undefined-global
    report_log('Issuing document link resolve request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global

    local doc_link_resolve_result = vim.lsp.buf_request_sync(0, 'documentLink/resolve', {
        -- NOTE: We have to manually destructure the params here, not sure why
        range = doc_link.range,
        target = doc_link.target,
        tooltip = doc_link.tooltip,
        data = doc_link.data,
    })

    if not doc_link_resolve_result then
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid document link result returned: ' .. vim.inspect(doc_link_resolve_result) .. '\n')
    elseif doc_link_resolve_result and #doc_link_resolve_result >= 1 and doc_link_resolve_result[1].result then
        local results_file = io.open('RESULTS_FILE', 'w')
        if not results_file then
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            exit() ---@diagnostic disable-line: undefined-global
        end

        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(doc_link_resolve_result[1].result, { escape_slash = true }))
        results_file:close()
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        mark_empty_file() ---@diagnostic disable-line: undefined-global
    end
    exit() ---@diagnostic disable-line: undefined-global
end
