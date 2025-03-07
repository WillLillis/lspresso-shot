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

-- TODO: This could definitely use some unit tests

--- Extracts the relative path from a file:// URI
---@param uri string
---@return string
---@diagnostic disable-next-line: unused-local, unused-function
local function extract_relative_path(uri)
    local path = nil
    -- TODO: Check for other URI schemes?
    if string.sub(uri, 1, 7) == 'file://' then
        path = vim.uri_to_fname(uri)
    else
        path = uri
    end
    -- Only strip the start if the server returns an absolute path
    if string.sub(path, 1, string.len('PARENT_PATH')) == 'PARENT_PATH' then
        return string.sub(path,
            string.len('PARENT_PATH') + 1,
            string.len(path))
    else
        return path
    end
end
