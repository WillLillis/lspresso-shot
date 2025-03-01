local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count < PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end
    report_log('Issuing completion request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local completion_result = vim.lsp.buf_request_sync(0, 'textDocument/completion', {
        textDocument = vim.lsp.util.make_text_document_params(0),
        ---@diagnostic disable-next-line: undefined-global
        SET_CURSOR_POSITION,
    }, 1000)
    if completion_result and #completion_result >= 1 and completion_result[1].result
        and (completion_result[1].result.items or #completion_result[1].result >= 1) then
        local results_file = io.open('RESULTS_FILE', "w")
        if not results_file then
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            vim.cmd('qa!')
        end
        local items = nil
        if completion_result[1].result.items then
            -- `CompletionResponse::List`
            items = completion_result[1].result.items
        else
            -- `CompletionResponse::Array`
            items = completion_result[1].result
        end
        for _, item in ipairs(items) do
            if item.documentation and item.documentation.value then
                item.documentation.value = string.gsub(item.documentation.value, "\\\\", "\\") -- HACK: find a better way?
            end
        end
        -- HACK: Does this ever return more than one??? For now, let's just grab the first
        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(completion_result[1].result))
        results_file:close()
        ---@diagnostic enable: need-check-nil
        vim.cmd('qa!')
    else
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid completion result returned: ' .. vim.inspect(completion_result) .. '\n')
    end
end
