local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    report_log('Issuing definition request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local definition_results = vim.lsp.buf_request_sync(0, "textDocument/definition", {
        textDocument = vim.lsp.util.make_text_document_params(0),
        ---@diagnostic disable-next-line: undefined-global
        SET_CURSOR_POSITION
    }, 1000)
    local results_file = io.open('RESULTS_FILE', "w")
    if not results_file then
        report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
        vim.cmd('qa!')
    end
    if definition_results and #definition_results > 0 and definition_results[1].result and #definition_results[1].result > 0 then
        local accum = '[\n'
        for _, def in ipairs(definition_results) do
            if def.result then
                for _, res in ipairs(def.result) do
                    if res.targetUri then
                        report_log('Setting `result.targetUri` field to relative path\n') ---@diagnostic disable-line: undefined-global
                        res.targetUri = extract_relative_path(res.targetUri) ---@diagnostic disable-line: undefined-global
                    end
                    accum = accum .. vim.json.encode(res) .. ',\n'
                end
            end
        end
        if string.len(accum) > 2 then
            accum = string.sub(accum, 1, string.len(accum) - 2) -- Remove the trailing comma
        end
        accum = accum .. '\n]'
        ---@diagnostic disable: need-check-nil
        results_file:write(accum)
        results_file:close()
        ---@diagnostic enable: need-check-nil
        ---@diagnostic disable-next-line: undefined-global, exp-in-action
        PROGRESS_EXIT_ACTION
    else
        ---@diagnostic disable: undefined-global
        report_log('No definition result returned (Attempt ' ..
            tostring(progress_count) .. '):\n ' .. vim.inspect(definition_results) .. '\n\n')
        ---@diagnostic enable: undefined-global
    end
    progress_count = progress_count + 1
end
