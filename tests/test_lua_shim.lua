--- Minimal test harness for the Lua shim, runs under plain LuaJIT.
--- Mocks the `vim` global so we can require the plugin modules without Neovim.

local passed, failed = 0, 0

local function test(name, fn)
  local ok, err = pcall(fn)
  if ok then
    passed = passed + 1
    print("  PASS  " .. name)
  else
    failed = failed + 1
    print("  FAIL  " .. name .. "\n        " .. tostring(err))
  end
end

local function assert_eq(a, b, msg)
  if a ~= b then
    error((msg or "assert_eq") .. ": expected " .. tostring(b) .. ", got " .. tostring(a), 2)
  end
end

---------------------------------------------------------------------------
-- vim mock
---------------------------------------------------------------------------

local autocmds_created = {}
local jobs_started = {}
local jobs_stopped = {}
local notify_calls = {}
local next_job_id = 1

local function reset_mock()
  autocmds_created = {}
  jobs_started = {}
  jobs_stopped = {}
  notify_calls = {}
  next_job_id = 1

  -- Clear cached modules so each test gets a fresh require
  package.loaded["im-select-ssh"] = nil
  package.loaded["im-select-ssh.config"] = nil
  package.loaded["im-select-ssh.tunnel"] = nil
end

_G.vim = {
  deepcopy = function(t)
    local out = {}
    for k, v in pairs(t) do
      if type(v) == "table" then
        out[k] = _G.vim.deepcopy(v)
      else
        out[k] = v
      end
    end
    return out
  end,

  tbl_deep_extend = function(behavior, ...)
    local result = {}
    for _, tbl in ipairs({ ... }) do
      for k, v in pairs(tbl) do
        if v ~= _G.vim.NIL then
          result[k] = v
        end
      end
    end
    return result
  end,

  env = {},

  log = { levels = { ERROR = 1, WARN = 2, INFO = 3 } },

  notify = function(msg, level)
    table.insert(notify_calls, { msg = msg, level = level })
  end,

  fn = {
    jobstart = function(cmd, opts)
      local id = next_job_id
      next_job_id = next_job_id + 1
      table.insert(jobs_started, { id = id, cmd = cmd, opts = opts })
      return id
    end,
    jobstop = function(id)
      table.insert(jobs_stopped, id)
    end,
  },

  api = {
    nvim_create_augroup = function(name, opts)
      return name
    end,
    nvim_create_autocmd = function(event, opts)
      table.insert(autocmds_created, { event = event, group = opts.group, callback = opts.callback })
    end,
  },

  g = {},

  uv = {
    random = function(n)
      local bytes = {}
      for i = 1, n do bytes[i] = string.char(math.random(0, 255)) end
      return table.concat(bytes)
    end,
    hrtime = function() return 123456789 end,
    os_getpid = function() return 42 end,
  },
}

-- Add lua/ to package path so require("im-select-ssh...") works
package.path = "./lua/?.lua;" .. "./lua/?/init.lua;" .. package.path

---------------------------------------------------------------------------
-- Tests: config
---------------------------------------------------------------------------

print("\n--- config ---")

reset_mock()
test("defaults include all expected keys", function()
  local config = require("im-select-ssh.config")
  assert_eq(config.defaults.server_bin, "im-select-server")
  assert_eq(config.defaults.tunnel_port, 9876)
  assert_eq(config.defaults.default_ime, "1033")
  assert_eq(config.defaults.client_host, nil)
  assert_eq(config.defaults.pin, nil)
end)

reset_mock()
test("apply merges user opts over defaults", function()
  local config = require("im-select-ssh.config")
  config.apply({ tunnel_port = 1234, client_host = "10.0.0.1" })
  assert_eq(config.current.tunnel_port, 1234)
  assert_eq(config.current.client_host, "10.0.0.1")
  assert_eq(config.current.server_bin, "im-select-server") -- unchanged default
end)

---------------------------------------------------------------------------
-- Tests: tunnel
---------------------------------------------------------------------------

print("\n--- tunnel ---")

reset_mock()
test("start resolves host from cfg.client_host", function()
  local tunnel = require("im-select-ssh.tunnel")
  tunnel.start({ client_host = "192.168.1.1", tunnel_port = 9876 })
  assert_eq(#jobs_started, 1)
  local cmd = jobs_started[1].cmd
  assert_eq(cmd[1], "ssh")
  -- Check the -R argument contains the port
  assert(cmd[3]:find("9876"), "-R arg should contain port")
  -- Last arg is the host
  assert_eq(cmd[#cmd], "192.168.1.1")
  assert(tunnel.job_id ~= nil, "job_id should be set")
end)

reset_mock()
test("start includes BatchMode, ConnectTimeout, StrictHostKeyChecking", function()
  local tunnel = require("im-select-ssh.tunnel")
  tunnel.start({ client_host = "host", tunnel_port = 9876 })
  local cmd = jobs_started[1].cmd
  local joined = table.concat(cmd, " ")
  assert(joined:find("BatchMode=yes"), "should include BatchMode=yes")
  assert(joined:find("ConnectTimeout=10"), "should include ConnectTimeout=10")
  assert(joined:find("StrictHostKeyChecking=yes"), "should include StrictHostKeyChecking=yes")
end)

reset_mock()
test("start stops existing tunnel before starting new one", function()
  local tunnel = require("im-select-ssh.tunnel")
  tunnel.start({ client_host = "host1", tunnel_port = 9876 })
  local first_id = tunnel.job_id
  tunnel.start({ client_host = "host2", tunnel_port = 9876 })
  -- Should have stopped the first tunnel
  assert_eq(#jobs_stopped, 1, "first tunnel should be stopped")
  assert_eq(jobs_stopped[1], first_id)
  -- New tunnel should be running
  assert(tunnel.job_id ~= first_id, "job_id should be updated")
end)

reset_mock()
test("on_exit only clears job_id when matching current job", function()
  local tunnel = require("im-select-ssh.tunnel")
  tunnel.start({ client_host = "host", tunnel_port = 9876 })
  local first_job = jobs_started[1]
  -- Simulate starting a second tunnel (without going through start, to test on_exit isolation)
  tunnel.job_id = 99  -- pretend a newer tunnel is now tracked
  -- Fire on_exit for the first (stale) job
  first_job.opts.on_exit(first_job.id, 0)
  -- job_id should NOT be cleared because the exiting job doesn't match
  assert_eq(tunnel.job_id, 99, "should not clear job_id for stale exit")
end)

reset_mock()
test("start resolves host from $SSH_CLIENT", function()
  _G.vim.env.SSH_CLIENT = "10.0.0.5 54321 22"
  local tunnel = require("im-select-ssh.tunnel")
  tunnel.start({ tunnel_port = 9876 })
  assert_eq(#jobs_started, 1)
  assert_eq(jobs_started[1].cmd[#jobs_started[1].cmd], "10.0.0.5")
  _G.vim.env.SSH_CLIENT = nil
end)

reset_mock()
test("start errors when no host available", function()
  _G.vim.env.SSH_CLIENT = nil
  local tunnel = require("im-select-ssh.tunnel")
  tunnel.start({ tunnel_port = 9876 })
  assert_eq(#jobs_started, 0, "should not start a job")
  assert_eq(#notify_calls, 1, "should notify error")
  assert_eq(notify_calls[1].level, _G.vim.log.levels.ERROR)
end)

reset_mock()
test("stop kills the job and clears job_id", function()
  local tunnel = require("im-select-ssh.tunnel")
  tunnel.start({ client_host = "host", tunnel_port = 9876 })
  local id = tunnel.job_id
  tunnel.stop()
  assert_eq(#jobs_stopped, 1)
  assert_eq(jobs_stopped[1], id)
  assert_eq(tunnel.job_id, nil)
end)

reset_mock()
test("stop is safe to call when no tunnel running", function()
  local tunnel = require("im-select-ssh.tunnel")
  tunnel.stop() -- should not error
  assert_eq(#jobs_stopped, 0)
end)

---------------------------------------------------------------------------
-- Tests: init (setup)
---------------------------------------------------------------------------

print("\n--- init.setup ---")

reset_mock()
test("setup generates a 6-digit PIN when none provided", function()
  _G.vim.env.SSH_CLIENT = "10.0.0.1 12345 22"
  local plugin = require("im-select-ssh")
  plugin.setup({})
  local config = require("im-select-ssh.config")
  assert(config.current.pin ~= nil, "pin should be set")
  assert_eq(#config.current.pin, 6, "pin should be 6 digits")
  assert(tonumber(config.current.pin), "pin should be numeric")
  _G.vim.env.SSH_CLIENT = nil
end)

reset_mock()
test("setup uses user-provided PIN", function()
  _G.vim.env.SSH_CLIENT = "10.0.0.1 12345 22"
  local plugin = require("im-select-ssh")
  plugin.setup({ pin = "abc123" })
  local config = require("im-select-ssh.config")
  assert_eq(config.current.pin, "abc123")
  _G.vim.env.SSH_CLIENT = nil
end)

reset_mock()
test("setup creates InsertLeave, InsertEnter, and VimLeavePre autocmds", function()
  _G.vim.env.SSH_CLIENT = "10.0.0.1 12345 22"
  local plugin = require("im-select-ssh")
  plugin.setup({})

  local events = {}
  for _, ac in ipairs(autocmds_created) do
    events[ac.event] = true
  end
  assert(events["InsertLeave"], "InsertLeave autocmd missing")
  assert(events["InsertEnter"], "InsertEnter autocmd missing")
  assert(events["VimLeavePre"], "VimLeavePre autocmd missing")
  _G.vim.env.SSH_CLIENT = nil
end)

reset_mock()
test("setup starts the SSH tunnel", function()
  _G.vim.env.SSH_CLIENT = "10.0.0.1 12345 22"
  local plugin = require("im-select-ssh")
  plugin.setup({})
  -- tunnel.start should have called jobstart (the first job)
  assert(#jobs_started >= 1, "tunnel job should be started")
  assert_eq(jobs_started[1].cmd[1], "ssh")
  _G.vim.env.SSH_CLIENT = nil
end)

reset_mock()
test("InsertLeave callback invokes server with save_and_switch and --pin", function()
  _G.vim.env.SSH_CLIENT = "10.0.0.1 12345 22"
  local plugin = require("im-select-ssh")
  plugin.setup({})
  local config = require("im-select-ssh.config")

  -- Find and fire the InsertLeave callback
  local leave_cb
  for _, ac in ipairs(autocmds_created) do
    if ac.event == "InsertLeave" then leave_cb = ac.callback end
  end
  assert(leave_cb, "InsertLeave callback should exist")

  local before = #jobs_started
  leave_cb()
  assert_eq(#jobs_started, before + 1)

  local cmd = jobs_started[#jobs_started].cmd
  assert_eq(cmd[1], "im-select-server")
  assert_eq(cmd[2], "save_and_switch")
  assert_eq(cmd[3], "--port")
  assert_eq(cmd[4], "9876")
  assert_eq(cmd[5], "--pin")
  assert_eq(cmd[6], config.current.pin)
  _G.vim.env.SSH_CLIENT = nil
end)

reset_mock()
test("InsertEnter callback invokes server with restore and --pin", function()
  _G.vim.env.SSH_CLIENT = "10.0.0.1 12345 22"
  local plugin = require("im-select-ssh")
  plugin.setup({})
  local config = require("im-select-ssh.config")

  local enter_cb
  for _, ac in ipairs(autocmds_created) do
    if ac.event == "InsertEnter" then enter_cb = ac.callback end
  end
  assert(enter_cb, "InsertEnter callback should exist")

  local before = #jobs_started
  enter_cb()
  assert_eq(#jobs_started, before + 1)

  local cmd = jobs_started[#jobs_started].cmd
  assert_eq(cmd[1], "im-select-server")
  assert_eq(cmd[2], "restore")
  assert_eq(cmd[5], "--pin")
  assert_eq(cmd[6], config.current.pin)
  _G.vim.env.SSH_CLIENT = nil
end)

reset_mock()
test("VimLeavePre callback stops the tunnel", function()
  _G.vim.env.SSH_CLIENT = "10.0.0.1 12345 22"
  local plugin = require("im-select-ssh")
  plugin.setup({})

  local leave_cb
  for _, ac in ipairs(autocmds_created) do
    if ac.event == "VimLeavePre" then leave_cb = ac.callback end
  end
  assert(leave_cb, "VimLeavePre callback should exist")

  leave_cb()
  assert(#jobs_stopped >= 1, "tunnel should be stopped")
  _G.vim.env.SSH_CLIENT = nil
end)

---------------------------------------------------------------------------
-- Summary
---------------------------------------------------------------------------

print(string.format("\n%d passed, %d failed", passed, failed))
if failed > 0 then os.exit(1) end
