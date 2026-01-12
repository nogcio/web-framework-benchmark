defmodule WfbPhoenix.Application do
  use Application

  def start(_type, _args) do
    children = [
      WfbPhoenix.Repo,
      {Phoenix.PubSub, name: WfbPhoenix.PubSub},
      WfbPhoenixWeb.Endpoint
    ]

    opts = [strategy: :one_for_one, name: WfbPhoenix.Supervisor]
    Supervisor.start_link(children, opts)
  end

  def config_change(changed, _new, removed) do
    WfbPhoenixWeb.Endpoint.config_change(changed, removed)
    :ok
  end
end
