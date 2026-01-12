defmodule WfbPhoenix.MixProject do
  use Mix.Project

  def project do
    [
      app: :wfb_phoenix,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  def application do
    [
      extra_applications: [:logger, :runtime_tools],
      mod: {WfbPhoenix.Application, []}
    ]
  end

  defp deps do
    [
      {:phoenix, "~> 1.8.3"},
      {:phoenix_ecto, "~> 4.7.0"},
      {:phoenix_pubsub, "~> 2.2.0"},
      {:ecto_sql, "~> 3.13.4"},
      {:postgrex, "~> 0.22.0"},
      {:jason, "~> 1.4.4"},
      {:bandit, "~> 1.10.1"}
    ]
  end
end
