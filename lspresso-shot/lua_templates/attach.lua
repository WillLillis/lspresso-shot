vim.filetype.add({
    extension = {
        FILE_EXTENSION = 'lspresso_shot',
    },
})

vim.api.nvim_create_autocmd('FileType', {
    pattern = 'lspresso_shot',
    callback = function(ev)
        if vim.bo[ev.buf].buftype == 'nofile' then
            report_error('Invalid buffer type opened') ---@diagnostic disable-line: undefined-global
            vim.cmd('qa!')
        end
        vim.lsp.start {
            name = 'lspresso_shot',
            cmd = { 'EXECUTABLE_PATH' },
            root_dir = 'ROOT_PATH/src',
            settings = {},
            capabilities = capabilities, ---@diagnostic disable-line: undefined-global
            on_attach = function(client, _) ---@diagnostic disable-line: unused-local
                ---@diagnostic disable-next-line: undefined-global, exp-in-action
                LSP_ACTION
            end,
        }
    end,
})
