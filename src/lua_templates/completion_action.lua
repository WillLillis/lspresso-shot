local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    report_log('Issuing completion request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local completion_result = vim.lsp.buf_request_sync(0, 'textDocument/completion', {
        textDocument = vim.lsp.util.make_text_document_params(0),
        ---@diagnostic disable-next-line: undefined-global
        SET_CURSOR_POSITION
    }, 1000)
    local results_file = io.open('RESULTS_FILE', "w")
    if not results_file then
        report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
        vim.cmd('qa!')
    end
    if completion_result and #completion_result >= 1 then
        for _, comp in ipairs(completion_result) do
            if comp.result then
                for _, item in ipairs(comp.result.items) do
                    if item.documentation and item.documentation.value then
                        item.documentation.value = string.gsub(item.documentation.value, "\\\\", "\\") -- HACK: find a better way?
                    end
                end
            end
        end
        -- Does this ever return more than one??? For now, let's just grab the first
        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(completion_result[1].result))
        results_file:close()
        ---@diagnostic enable: need-check-nil
        ---@diagnostic disable-next-line: undefined-global, exp-in-action
        PROGRESS_EXIT_ACTION
    else
        ---@diagnostic disable: undefined-global
        report_log('No completion result returned (Attempt ' ..
            tostring(progress_count) .. '):\n ' .. vim.inspect(completion_result) .. '\n\n')
        ---@diagnostic enable: undefined-global
    end
    progress_count = progress_count + 1
end
