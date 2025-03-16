local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count ~= PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end
    report_log('Issuing document link request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local doc_link_result = vim.lsp.buf_request_sync(0, 'textDocument/documentLink', {
        textDocument = vim.lsp.util.make_text_document_params(0),
    }, 1000)

    if not doc_link_result then
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid document link result returned: ' .. vim.inspect(doc_link_result) .. '\n')
    elseif doc_link_result and #doc_link_result >= 1 and doc_link_result[1].result then
        local results_file = io.open('RESULTS_FILE', 'w')
        if not results_file then
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            vim.cmd('qa!')
        end

        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(doc_link_result[1].result, { escape_slash = true }))
        results_file:close()
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        mark_empty_file() ---@diagnostic disable-line: undefined-global
    end
    vim.cmd('qa!')
end
