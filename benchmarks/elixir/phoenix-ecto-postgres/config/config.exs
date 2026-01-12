import Config

config :wfb_phoenix,
  ecto_repos: [WfbPhoenix.Repo],
  generators: [timestamp_type: :utc_datetime]

config :wfb_phoenix, WfbPhoenixWeb.Endpoint,
  url: [host: "localhost"],
  adapter: Bandit.PhoenixAdapter,
  render_errors: [
    formats: [json: WfbPhoenixWeb.ErrorJSON],
    layout: false
  ],
  pubsub_server: WfbPhoenix.PubSub,
  live_view: [signing_salt: "SECRET_SALT"] # Required or not? Minimal config.

config :logger, level: :info

import_config "#{config_env()}.exs"
