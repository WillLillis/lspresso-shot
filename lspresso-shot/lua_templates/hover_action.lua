local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count < PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end
    report_log('Issuing hover request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local hover_result = vim.lsp.buf_request_sync(0, 'textDocument/hover', {
        textDocument = vim.lsp.util.make_text_document_params(0),
        ---@diagnostic disable-next-line: undefined-global
        SET_CURSOR_POSITION,
    }, 1000)
    if hover_result and #hover_result >= 1 and hover_result[1].result and hover_result[1].result.range then
        local results_file = io.open('RESULTS_FILE', 'w')
        if not results_file then
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            vim.cmd('qa!')
        end
        local cleaned = hover_result[1].result
        if type(cleaned.contents) == "string" then
            cleaned.contents = string.gsub(cleaned.contents, "\\\\", "\\")
        elseif type(cleaned.contents) == "table" and type(cleaned.contents.value) == "string" then
            -- seems like a false positive, but maybe I'm doing something wrong?
            ---@diagnostic disable-next-line: inject-field
            cleaned.contents.value = string.gsub(cleaned.contents.value, "\\\\", "\\")
        elseif #cleaned.contents >= 1 then
            -- seems like a false positive, but maybe I'm doing something wrong?
            ---@diagnostic disable-next-line: param-type-mismatch
            for _, val in ipairs(cleaned.contents) do
                if type(val) == "string" then
                    -- `HoverContents::Array(Vec<MarkedString::String>)`
                    val = string.gsub(val, "\\\\", "\\")
                elseif type(val.value) == "string" then
                    -- `HoverContents::Array(Vec<MarkedString::LanguageString>)`
                    val.value = string.gsub(val.value, "\\\\", "\\")
                end
            end
        end
        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(cleaned))
        results_file:close()
        vim.cmd('qa!')
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid hover result returned: ' .. vim.inspect(hover_result) .. '\n')
    end
end
