local M = {}

M.job_id = nil

function M.start(cfg)
  local host = cfg.client_host
  if not host then
    local ssh_client = vim.env.SSH_CLIENT
    if ssh_client then
      host = ssh_client:match("^(%S+)")
    end
  end

  if not host then
    vim.notify("[im-select-ssh] Cannot determine client host: $SSH_CLIENT not set and client_host not configured", vim.log.levels.ERROR)
    return
  end

  local port = tostring(cfg.tunnel_port)
  M.job_id = vim.fn.jobstart({
    "ssh",
    "-R", port .. ":localhost:" .. port,
    "-N",
    "-o", "ExitOnForwardFailure=yes",
    host,
  }, {
    on_exit = function(_, code)
      if code ~= 0 and M.job_id then
        vim.notify("[im-select-ssh] SSH tunnel exited with code " .. code, vim.log.levels.WARN)
      end
      M.job_id = nil
    end,
  })

  if M.job_id <= 0 then
    vim.notify("[im-select-ssh] Failed to start SSH tunnel", vim.log.levels.ERROR)
    M.job_id = nil
  end
end

function M.stop()
  if M.job_id then
    vim.fn.jobstop(M.job_id)
    M.job_id = nil
  end
end

return M
