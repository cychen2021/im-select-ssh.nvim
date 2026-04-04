local M = {}

local config = require("im-select-ssh.config")

function M.setup(opts)
  config.apply(opts or {})

  local group = vim.api.nvim_create_augroup("ImSelectSsh", { clear = true })

  vim.api.nvim_create_autocmd("InsertLeave", {
    group = group,
    callback = function()
      -- TODO: invoke im-select-server with "save_and_switch"
    end,
  })

  vim.api.nvim_create_autocmd("InsertEnter", {
    group = group,
    callback = function()
      -- TODO: invoke im-select-server with "restore"
    end,
  })
end

return M
