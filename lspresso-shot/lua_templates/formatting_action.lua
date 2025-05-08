local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count ~= PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end

    local params = {}

    report_log('Issuing formatting request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global

    if INVOKE_FORMAT then ---@diagnostic disable-line: undefined-global
        local results_file = io.open('RESULTS_FILE', 'w')
        if not results_file then
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            exit() ---@diagnostic disable-line: undefined-global
        end
        ---@diagnostic disable-next-line: undefined-global, exp-in-action
        PARAM_ASSIGN

        report_log('Format opts: ' .. tostring(vim.inspect(params)) .. '\n') ---@diagnostic disable-line: undefined-global
        vim.lsp.buf.format({
            async = false,
            formatting_options = params.formatting_options,
        })
        local lines = vim.api.nvim_buf_get_lines(0, 0, -1, true)
        local formatted = table.concat(lines, "\\n")
        ---@diagnostic disable: need-check-nil
        results_file:write('"' .. formatted .. '"')
        results_file:close()
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global, exp-in-action
        PARAM_ASSIGN

        report_log('Format opts: ' .. tostring(vim.inspect(params)) .. '\n') ---@diagnostic disable-line: undefined-global

        local formatting_result = vim.lsp.buf_request_sync(0, 'textDocument/formatting', {
            textDocument = vim.lsp.util.make_text_document_params(0),
            options = params.options
        })
        if not formatting_result then
            ---@diagnostic disable-next-line: undefined-global
            report_log('No valid formatting result returned: ' .. vim.inspect(formatting_result) .. '\n')
        elseif formatting_result and #formatting_result >= 1 and formatting_result[1].result then
            local results_file = io.open('RESULTS_FILE', 'w')
            if not results_file then
                report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
                exit() ---@diagnostic disable-line: undefined-global
            end
            ---@diagnostic disable: need-check-nil
            results_file:write(vim.json.encode(formatting_result[1].result, { escape_slash = true }))
            results_file:close()
            ---@diagnostic enable: need-check-nil
        else
            ---@diagnostic disable-next-line: undefined-global
            mark_empty_file() ---@diagnostic disable-line: undefined-global
        end
    end
    exit() ---@diagnostic disable-line: undefined-global
end
