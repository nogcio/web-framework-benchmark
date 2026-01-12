defmodule WfbPhoenix.Models.Post do
  use Ecto.Schema

  schema "posts" do
    field :title, :string
    field :content, :string
    field :views, :integer
    field :created_at, :naive_datetime

    belongs_to :user, WfbPhoenix.Models.User
  end
end
