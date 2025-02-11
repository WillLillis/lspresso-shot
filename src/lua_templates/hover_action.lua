local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable: unused-function, unused-local, undefined-global
local function check_progress_result()
    local hover_result = vim.lsp.buf_request_sync(0, 'textDocument/hover', {
        textDocument = vim.lsp.util.make_text_document_params(0),
        SET_CURSOR_POSITION
    }, 1000)
    -- Write the results in a JSON format for easy deserialization
    local file = io.open('RESULTS_FILE', 'w')
    if hover_result and #hover_result >= 1 and hover_result[1].result and file then
        local cleaned = hover_result[1]
        cleaned.result.contents.value = string.gsub(cleaned.result.contents.value, "\\\\", "\\") -- HACK, find a better way
        file:write(vim.json.encode(cleaned.result))
        file:close()
        vim.cmd('qa!')
    else
        report_log('No hover result returned (Attempt ' ..
            tostring(progress_count) .. '):\n ' .. vim.inspect(hover_result) .. '\n\n')
    end
    progress_count = progress_count + 1
end
---@diagnostic enable: unused-function, unused-local, undefined-global
