local progress_count = 0 -- track how many times we've tried for the logs

vim.api.nvim_create_autocmd('DiagnosticChanged', {
    callback = function(_)
        local diagnostics_result = vim.diagnostic.get(0, {})
        local file = io.open('RESULTS_FILE', 'w')
        if diagnostics_result and #diagnostics_result >= 1 and file then
            local num_diagnostics = #diagnostics_result
            local accum = '[\n' -- Open the array
            for i, diagnostic in pairs(diagnostics_result) do
                local result = diagnostic.user_data.lsp
                result.message = string.gsub(result.message, "\\\\", "\\") -- HACK, find a better way
                if result.relatedInformation then
                    for info_idx, info in pairs(result.relatedInformation) do
                        if info.location.uri then
                            ---@diagnostic disable: undefined-global
                            result.relatedInformation[info_idx].location.uri = extract_relative_path(info.location.uri)
                            ---@diagnostic enable: undefined-global
                        end
                    end
                end
                accum = accum .. vim.json.encode(result)
                if i < num_diagnostics then
                    accum = accum .. ',\n'
                end
            end
            accum = accum .. '\n]' -- Close the array

            file:write(accum)
            file:close()
            vim.cmd('qa!')
        else
            ---@diagnostic disable: undefined-global
            report_log('No diagnostic result returned (Attempt ' ..
                tostring(progress_count) .. '):\n ' .. vim.inspect(diagnostics_result) .. '\n\n')
            ---@diagnostic enable: undefined-global
        end
        progress_count = progress_count + 1
    end,
})
