local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable: unused-function, unused-local, undefined-global
local function check_progress_result()
    local definition_results = vim.lsp.buf_request_sync(0, "textDocument/definition", {
        textDocument = vim.lsp.util.make_text_document_params(0),
        SET_CURSOR_POSITION
    }, 1000)
    local file = io.open('RESULTS_FILE', "w")
    if file and definition_results and #definition_results > 0 and definition_results[1].result and #definition_results[1].result > 0 then
        local accum = '[\n' -- Open the array
        for def_idx, def in ipairs(definition_results) do
            if def.result then
                for _, res in ipairs(def.result) do
                    if res.targetUri then
                        res.targetUri = extract_relative_path(res.targetUri)
                    end
                    accum = accum .. vim.json.encode(res) .. ',\n'
                end
            end
        end
        if string.len(accum) > 2 then
            accum = string.sub(accum, 1, string.len(accum) - 2) -- Remove the trailing comma
        end
        accum = accum .. '\n]' -- Close the array
        file:write(accum)
        file:close()
        vim.cmd('qa!')
    else
        report_log('No definition result returned (Attempt ' ..
            tostring(progress_count) .. '):\n ' .. vim.inspect(definition_results) .. '\n\n')
    end
    progress_count = progress_count + 1
end
---@diagnostic enable: unused-function, unused-local, undefined-global
