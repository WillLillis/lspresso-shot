local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count < PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end
    report_log('Issuing document symbol request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local doc_sym_result = vim.lsp.buf_request_sync(0, 'textDocument/documentSymbol', {
        textDocument = vim.lsp.util.make_text_document_params(0),
        ---@diagnostic disable-next-line: undefined-global
    }, 1000)
    if doc_sym_result and #doc_sym_result >= 1 and doc_sym_result[1].result then
        local results_file = io.open('RESULTS_FILE', 'w')
        if not results_file then
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            vim.cmd('qa!')
        end
        for _, sym in ipairs(doc_sym_result[1].result) do
            if sym.location and sym.location.uri then
                ---@diagnostic disable-next-line: undefined-global
                sym.location.uri = extract_relative_path(sym.location.uri)
            end
        end

        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(doc_sym_result[1].result))
        results_file:close()
        vim.cmd('qa!')
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid hover result returned: ' .. vim.inspect(doc_sym_result) .. '\n')
    end
end
