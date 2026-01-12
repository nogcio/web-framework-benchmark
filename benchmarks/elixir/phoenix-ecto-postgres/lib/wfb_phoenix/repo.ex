defmodule WfbPhoenix.Repo do
  use Ecto.Repo,
    otp_app: :wfb_phoenix,
    adapter: Ecto.Adapters.Postgres
end
