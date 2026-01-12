defmodule WfbPhoenixWeb.HealthController do
  use WfbPhoenixWeb, :controller

  def index(conn, _params) do
    # Verify DB connection
    case Ecto.Adapters.SQL.query(WfbPhoenix.Repo, "SELECT 1") do
      {:ok, _} -> send_resp(conn, 200, "OK")
      _ -> send_resp(conn, 503, "Service Unavailable")
    end
  end
end
