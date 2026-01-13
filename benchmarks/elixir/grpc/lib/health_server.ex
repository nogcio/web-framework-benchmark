defmodule HealthServer do
  use GRPC.Server, service: Grpc.Health.V1.Health.Service

  @spec check(Grpc.Health.V1.HealthCheckRequest.t, GRPC.Server.Stream.t) :: Grpc.Health.V1.HealthCheckResponse.t
  def check(_request, _stream) do
    Grpc.Health.V1.HealthCheckResponse.new(status: :SERVING)
  end

  @spec watch(Grpc.Health.V1.HealthCheckRequest.t, GRPC.Server.Stream.t) :: any
  def watch(_request, stream) do
    # Simple implementation just to pass requirements
    GRPC.Server.send_reply(stream, Grpc.Health.V1.HealthCheckResponse.new(status: :SERVING))
  end
end
