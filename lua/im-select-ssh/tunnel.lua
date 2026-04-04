local M = {}

function M.start(port)
  -- TODO: Start SSH reverse tunnel (ssh -R port:localhost:port ...)
end

function M.stop()
  -- TODO: Tear down the tunnel (kill the ssh process)
end

return M
