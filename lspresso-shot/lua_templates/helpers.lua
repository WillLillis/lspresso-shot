---@param msg string
---@diagnostic disable-next-line: unused-local, unused-function
local function report_error(msg)
    local error_file = io.open('ERROR_PATH', 'a')
    if error_file then
        error_file:write(msg)
        error_file:close()
    end
end

---@param msg string
---@diagnostic disable-next-line: unused-local, unused-function
local function report_log(msg)
    local log_file = io.open('LOG_PATH', 'a')
    if log_file then
        log_file:write(msg)
        log_file:close()
    end
end

--- Creates `empty` in the test case root directory
---@diagnostic disable-next-line: unused-local, unused-function
local function mark_empty_file()
    local empty_file = io.open('EMPTY_PATH', "w")
    if not empty_file then
        report_error('Could not open empty file') ---@diagnostic disable-line: undefined-global
        vim.cmd('qa!')
    end
    ---@diagnostic disable: need-check-nil
    empty_file:write('')
    empty_file:close()
    ---@diagnostic enable: need-check-nil
end

---@param time_ns number
---@diagnostic disable-next-line: unused-local, unused-function
local function record_benchmark_result(time_ns)
    local benchmark_file, err = io.open('BENCHMARK_PATH', 'a')
    if not benchmark_file then
        report_error('Could not open benchmark file: ' .. err)
    else
        benchmark_file:write(tostring(time_ns) .. '\n')
        benchmark_file:close()
    end
end

local messages = {}

local original_notify = vim.notify
---@diagnostic disable-next-line: duplicate-set-field
vim.notify = function(message, log_level, opts)
    table.insert(messages, message)
    return original_notify(message, log_level, opts)
end

---@diagnostic disable-next-line: unused-local, unused-function
local function exit()
    for _, message in ipairs(messages) do
        report_error(message)
    end
    vim.cmd('qa!')
end

---@diagnostic disable-next-line: unused-local, unused-function
local function timeout_exit()
    report_error('Timeout of `TIMEOUT_MS`ms exceeded')
    local timeout_file, err = io.open('TIMEOUT_PATH', 'w')
    if not timeout_file then
        report_error('Failed not open timeout file: ' .. err)
        exit()
    else
        timeout_file:write('')
        timeout_file:close()
    end
    exit()
end


local capabilities = vim.lsp.protocol.make_client_capabilities()
capabilities.experimental = {
    commands = {
        commands = {
            ---@diagnostic disable-next-line: undefined-global
            COMMANDS
        },
    },
}

vim.lsp.log.set_format_func(function(msg)
    report_log('LSP LOG: ' .. msg)
    return nil
end)

local timer, err = vim.uv.new_timer()
if err then
    report_log('Failed to create timeout timer: ' .. tostring(err)) ---@diagnostic disable-line: undefined-global
elseif timer then
    ---@diagnostic disable-next-line: undefined-global
    timer:start(TIMEOUT_MS, 0, vim.schedule_wrap(timeout_exit))
end
