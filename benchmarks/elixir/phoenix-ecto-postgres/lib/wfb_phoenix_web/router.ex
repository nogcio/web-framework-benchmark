defmodule WfbPhoenixWeb.Router do
  use WfbPhoenixWeb, :router

  pipeline :api do
    plug :accepts, ["json", "text"]
  end

  scope "/", WfbPhoenixWeb do
    pipe_through :api

    get "/health", HealthController, :index
    get "/plaintext", PlaintextController, :index
    post "/json/aggregate", JsonController, :aggregate
    
    # DB Complex
    get "/db/user-profile/:email", DbController, :user_profile
  end
end
