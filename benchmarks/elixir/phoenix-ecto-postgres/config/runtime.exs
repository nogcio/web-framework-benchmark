import Config

if System.get_env("PHX_SERVER") do
  config :wfb_phoenix, WfbPhoenixWeb.Endpoint, server: true
end

if config_env() == :prod do
  database_host = System.get_env("DB_HOST") || "localhost"
  database_port = (System.get_env("DB_PORT") || "5432") |> String.to_integer()
  database_user = System.get_env("DB_USER") || "benchmark"
  database_password = System.get_env("DB_PASSWORD") || "benchmark"

  database_name = System.get_env("DB_NAME") || "benchmark"
  pool_size = String.to_integer(System.get_env("DB_POOL_SIZE") || "256")

  IO.puts("CONNECTING TO DB: #{database_host}:#{database_port} (User: #{database_user}, DB: #{database_name})")

  config :wfb_phoenix, WfbPhoenix.Repo,
    username: database_user,
    password: database_password,
    hostname: database_host,
    port: database_port,
    database: database_name,
    pool_size: pool_size,
    socket_options: [:inet],
    show_sensitive_data_on_connection_error: false

  port = String.to_integer(System.get_env("PORT") || "8080")

  config :wfb_phoenix, WfbPhoenixWeb.Endpoint,
    http: [
      ip: {0, 0, 0, 0},
      port: port
    ],
    secret_key_base: "SUPER_SECRET_KEY_BASE_SHOULD_BE_LONG_ENOUGH_1234567890"

end
