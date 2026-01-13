defmodule GrpcBenchmark.Application do
  use Application

  @impl true
  def start(_type, _args) do
    port = String.to_integer(System.get_env("PORT") || "8080")
    
    children = [
      {GRPC.Server.Supervisor, endpoint: GrpcBenchmark.Endpoint, port: port, start_server: true}
    ]

    opts = [strategy: :one_for_one, name: GrpcBenchmark.Supervisor]
    Supervisor.start_link(children, opts)
  end
end

defmodule GrpcBenchmark.Endpoint do
  use GRPC.Endpoint

  # intercept GRPC.Server.Interceptors.Logger
  run AnalyticsService.Server
  run HealthServer
end
