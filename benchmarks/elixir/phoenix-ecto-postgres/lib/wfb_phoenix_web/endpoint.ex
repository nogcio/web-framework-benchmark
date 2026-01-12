defmodule WfbPhoenixWeb.Endpoint do
  use Phoenix.Endpoint, otp_app: :wfb_phoenix

  # The session will be stored in the cookie and signed,
  # this means its contents can be read but not tampered with.
  # Set :encryption_salt if you would also like to encrypt it.
  @session_options [
    store: :cookie,
    key: "_wfb_phoenix_key",
    signing_salt: "SECRET_SALT_SESSION"
  ]

  # Serve static files from benchmarks_data.
  # The spec says `/files/XXX`.
  # We map `/files` to the directory.
  # We assume the directory is accessible.
  
  # Note: logic to resolve DATA_DIR.
  # In runtime.exs we don't configure Plug.Static.
  # We can do it here. But `from` path must be valid.
  # We'll use "benchmarks_data" which is copied in Dockerfile.
  
  plug Plug.Static,
    at: "/files",
    from: "benchmarks_data",
    gzip: false,
    only: ~w(15kb.bin 1mb.bin 10mb.bin), # Allow serving these files
    # Range support is built-in to Plug.Static via `plug :match` and `plug :dispatch`? 
    # Actually Plug.Static handles sending the file.
    # We need to ensure ETag is generated. `etags: true` (default).
    headers: [{"access-control-allow-origin", "*"}] # Optional

  # Code reloading can be explicitly enabled under the
  # :code_reloader configuration of your endpoint.
  if code_reloading? do
    plug Phoenix.CodeReloader
    plug Phoenix.Ecto.CheckRepoStatus, otp_app: :wfb_phoenix
  end

  plug Plug.RequestId

  plug Plug.Parsers,
    parsers: [:urlencoded, :multipart, :json],
    pass: ["*/*"],
    json_decoder: Phoenix.json_library()

  plug Plug.MethodOverride
  plug Plug.Head
  plug Plug.Session, @session_options
  plug WfbPhoenixWeb.Router
end
