defmodule WfbPhoenixWeb do
  def controller do
    quote do
      use Phoenix.Controller, namespace: WfbPhoenixWeb

      import Plug.Conn
      alias WfbPhoenixWeb.Router.Helpers, as: Routes
    end
  end

  def view do
    quote do
      use Phoenix.View,
        root: "lib/wfb_phoenix_web/templates",
        namespace: WfbPhoenixWeb

      import Phoenix.Controller, only: [get_flash: 1, get_flash: 2, view_module: 1]
      alias WfbPhoenixWeb.Router.Helpers, as: Routes
    end
  end

  def router do
    quote do
      use Phoenix.Router
      import Plug.Conn
      import Phoenix.Controller
    end
  end

  def channel do
    quote do
      use Phoenix.Channel
    end
  end

  def static_paths, do: ~w(assets fonts images favicon.ico robots.txt)

  defmacro __using__(which) when is_atom(which) do
    apply(__MODULE__, which, [])
  end
end
