---@diagnostic disable: unused-local, unused-function
---@param msg string
local function report_error(msg)
    local error_file = io.open('ERROR_PATH', 'a')
    if error_file then
        error_file:write(msg)
        error_file:close()
    end
end
---@diagnostic enable: unused-function

---@diagnostic disable: unused-local, unused-function
---@param msg string
local function report_log(msg)
    local log_file = io.open('LOG_PATH', 'a')
    if log_file then
        log_file:write(msg)
        log_file:close()
    end
end
---@diagnostic enable: unused-function

---@diagnostic disable: unused-local, unused-function
--- Extracts the relative path from a file:// URI
---@param uri string
---@return string
local function extract_relative_path(uri)
    assert(string.sub(uri, 1, 7) == 'file://', 'URI is not a file:// URI')
    local path = vim.uri_to_fname(uri)
    return string.sub(path,
        string.len('PARENT_PATH') + 1,
        string.len(path))
end
---@diagnostic enable: unused-function

