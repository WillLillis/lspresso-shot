local progress_count = 0 -- track how many times we've tried for the logs

vim.api.nvim_create_autocmd('DiagnosticChanged', {
    callback = function(_)
        progress_count = progress_count + 1
        report_log('Issuing hover request (Attempt ' .. tostring(progress_count) .. ')\n') ---@diagnostic disable-line: undefined-global
        local diagnostics_result = vim.diagnostic.get(0, {})
        if diagnostics_result and #diagnostics_result >= 1 then
            local results_file = io.open('RESULTS_FILE', 'w')
            if not results_file then
                report_error('Could not open results file') ---@diagnostic disable-line: undefined-global
                vim.cmd('qa!')
            end
            local num_diagnostics = #diagnostics_result
            local accum = '[\n'
            for i, diagnostic in pairs(diagnostics_result) do
                report_log('Parsing diagnostic ' .. vim.inspect(diagnostic) .. '\n') ---@diagnostic disable-line: undefined-global
                local result = diagnostic.user_data.lsp
                result.message = string.gsub(result.message, "\\\\", "\\") -- HACK: find a better way?
                if result.relatedInformation then
                    for info_idx, info in pairs(result.relatedInformation) do
                        if info.location.uri then
                            report_log('Setting `location.uri` field to relative path\n') ---@diagnostic disable-line: undefined-global
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
            accum = accum .. '\n]'

            ---@diagnostic disable: need-check-nil
            results_file:write(accum)
            results_file:close()
            vim.cmd('qa!')
            ---@diagnostic enable: need-check-nil
        else
            ---@diagnostic disable: undefined-global
            report_log('No diagnostic result returned (Attempt ' ..
                tostring(progress_count) .. '):\n ' .. vim.inspect(diagnostics_result) .. '\n\n')
            ---@diagnostic enable: undefined-global
        end
    end,
})
