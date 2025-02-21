local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count < PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end

    -- Receive  json-encoded `FormattingOptions` from the rust side in `json_opts`
    ---@diagnostic disable-next-line: undefined-global, lowercase-global
    json_opts = [[
JSON_OPTIONS
    ]]
    format_opts = vim.json.decode(json_opts) ---@diagnostic disable-line: undefined-global, lowercase-global

    report_log('Format opts: ' .. tostring(vim.inspect(format_opts)) .. '\n') ---@diagnostic disable-line: undefined-global
    report_log('Issuing formatting request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global

    if INVOKE_FORMAT then ---@diagnostic disable-line: undefined-global
        local results_file = io.open('RESULTS_FILE', 'w')
        if not results_file then
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            vim.cmd('qa!')
        end
        vim.lsp.buf.format({
            async = false,
            formatting_options = format_opts
        })
        local lines = vim.api.nvim_buf_get_lines(0, 0, -1, true)
        local formatted = table.concat(lines, "\\n")
        ---@diagnostic disable: need-check-nil
        results_file:write('"' .. formatted .. '"')
        results_file:close()
        vim.cmd('qa!')
        ---@diagnostic enable: need-check-nil
    else
        local formatting_result = vim.lsp.buf_request_sync(0, 'textDocument/formatting', {
            textDocument = vim.lsp.util.make_text_document_params(0),
            options = format_opts
        }, 1000)
        if formatting_result and #formatting_result >= 1 and formatting_result[1].result and #formatting_result[1].result >= 1 then
            local results_file = io.open('RESULTS_FILE', 'w')
            if not results_file then
                report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
                vim.cmd('qa!')
            end
            --- NOTE: Does this ever return more than one result? Just use the first for now
            ---@diagnostic disable: need-check-nil
            results_file:write(vim.json.encode(formatting_result[1].result))
            results_file:close()
            vim.cmd('qa!')
            ---@diagnostic enable: need-check-nil
        else
            ---@diagnostic disable-next-line: undefined-global
            report_log('No valid formatting result returned: ' .. vim.inspect(formatting_result) .. '\n')
        end
    end
end
