local progress_count = 0 -- track how many times we've tried for the logs

---@diagnostic disable-next-line: unused-function, unused-local
local function check_progress_result()
    progress_count = progress_count + 1
    if progress_count ~= PROGRESS_THRESHOLD then ---@diagnostic disable-line: undefined-global
        report_log(tostring(progress_count) .. ' < ' .. tostring(PROGRESS_THRESHOLD) .. '\n') ---@diagnostic disable-line: undefined-global
        return
    end
    -- Receive  json-encoded `SignatureHelpContext` from the rust side in `json_ctx`
    local json_ctx = [[
SIGNATURE_CONTEXT
    ]]
    local sig_ctx = vim.json.decode(json_ctx)

    report_log('Signature context: ' .. tostring(vim.inspect(sig_ctx)) .. '\n') ---@diagnostic disable-line: undefined-global
    report_log('Issuing signature help request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
    local signature_help_result = vim.lsp.buf_request_sync(0, 'textDocument/signatureHelp', {
        textDocument = vim.lsp.util.make_text_document_params(0),
        ---@diagnostic disable: undefined-global
        SET_CURSOR_POSITION,
        context = sig_ctx,
        ---@diagnostic enable: undefined-global
    })

    if not signature_help_result then
        ---@diagnostic disable-next-line: undefined-global
        report_log('No valid signature help result returned: ' .. vim.inspect(signature_help_result) .. '\n')
    elseif signature_help_result and #signature_help_result >= 1 and signature_help_result[1].result then
        local results_file = io.open('RESULTS_FILE', 'w')
        if not results_file then
            report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
            exit() ---@diagnostic disable-line: undefined-global
        end
        ---@diagnostic disable: need-check-nil
        results_file:write(vim.json.encode(signature_help_result[1].result, { escape_slash = true }))
        results_file:close()
        ---@diagnostic enable: need-check-nil
    else
        ---@diagnostic disable-next-line: undefined-global
        mark_empty_file() ---@diagnostic disable-line: undefined-global
    end
    exit() ---@diagnostic disable-line: undefined-global
end
