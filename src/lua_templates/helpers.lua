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

--- Extracts the relative path from a file:// URI
---@param uri string
---@return string
---@diagnostic disable-next-line: unused-local, unused-function
local function extract_relative_path(uri)
    if not string.sub(uri, 1, 7) == 'file://' then
        report_error('URI is not a file:// URI')
    end
    local path = vim.uri_to_fname(uri)
    return string.sub(path,
        string.len('PARENT_PATH') + 1,
        string.len(path))
end
