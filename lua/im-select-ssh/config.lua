local M = {}

M.defaults = {
  server_bin = "im-select-server",
  tunnel_port = 9876,
  default_ime = "1033",
}

M.current = vim.deepcopy(M.defaults)

function M.apply(opts)
  M.current = vim.tbl_deep_extend("force", M.defaults, opts)
end

return M
