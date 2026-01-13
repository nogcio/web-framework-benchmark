defmodule GrpcBenchmark.MixProject do
  use Mix.Project

  def project do
    [
      app: :grpc_benchmark,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  def application do
    [
      extra_applications: [],
      mod: {GrpcBenchmark.Application, []}
    ]
  end

  defp deps do
    [
      {:grpc, "~> 0.9.0"},
      {:protobuf, "~> 0.12.0"}, 
      {:cowlib, "~> 2.11", override: true} # Sometimes needed for grpc compatibility
    ]
  end
end
