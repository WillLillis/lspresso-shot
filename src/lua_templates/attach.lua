vim.filetype.add({
    extension = {
        FILE_EXTENSION = 'lspresso_shot',
    },
})

vim.api.nvim_create_autocmd('FileType', {
    pattern = 'lspresso_shot',
    callback = function(ev)
        if vim.bo[ev.buf].buftype == 'nofile' then
            ---@diagnostic disable: undefined-global
            report_error('Invalid buffer type opened')
            ---@diagnostic enable: undefined-global
            vim.cmd('qa!')
        end
        vim.lsp.start {
            name = 'lspresso_shot',
            cmd = { 'EXECUTABLE_PATH' },
            root_dir = 'ROOT_PATH/src',
            settings = {},
            ---@diagnostic disable: unused-local
            on_attach = function(client, _)
                ---@diagnostic enable: unused-local
                ---@diagnostic disable: undefined-global, exp-in-action
                LSP_ACTION
                ---@diagnostic enable: undefined-global, exp-in-action
            end,
        }
    end,
})
