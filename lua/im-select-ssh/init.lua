local M = {}

local config = require("im-select-ssh.config")
local tunnel = require("im-select-ssh.tunnel")

local function generate_pin()
  local uv = vim.uv or vim.loop

  if uv and uv.random then
    local ok, bytes = pcall(uv.random, 4)
    if ok and type(bytes) == "string" and #bytes == 4 then
      local value = 0
      for i = 1, #bytes do
        value = (value * 256 + bytes:byte(i)) % 1000000
      end
      return string.format("%06d", value)
    end
  end

  local seed = tonumber(tostring(os.time()) .. tostring((uv and uv.hrtime and uv.hrtime()) or 0):sub(-6))
  seed = seed + ((uv and uv.os_getpid and uv.os_getpid()) or 0)
  math.randomseed(seed)
  return string.format("%06d", math.random(0, 999999))
end

function M.setup(opts)
  config.apply(opts or {})

  if not config.current.pin then
    config.current.pin = generate_pin()
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
