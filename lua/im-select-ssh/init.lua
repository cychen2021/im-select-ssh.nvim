local M = {}

local config = require("im-select-ssh.config")
local tunnel = require("im-select-ssh.tunnel")

function M.setup(opts)
  config.apply(opts or {})

  if not config.current.pin then
    config.current.pin = string.format("%06d", math.random(0, 999999))
  end

  tunnel.start(config.current)

  local group = vim.api.nvim_create_augroup("ImSelectSsh", { clear = true })
  local bin = config.current.server_bin
  local port = tostring(config.current.tunnel_port)
  local pin = config.current.pin

  vim.api.nvim_create_autocmd("InsertLeave", {
    group = group,
    callback = function()
      vim.fn.jobstart({ bin, "save_and_switch", "--port", port, "--pin", pin })
    end,
  })

  vim.api.nvim_create_autocmd("InsertEnter", {
    group = group,
    callback = function()
      vim.fn.jobstart({ bin, "restore", "--port", port, "--pin", pin })
    end,
  })

  vim.api.nvim_create_autocmd("VimLeavePre", {
    group = group,
    callback = function()
      tunnel.stop()
    end,
  })
end

return M
