defmodule WfbPhoenix.Models.User do
  use Ecto.Schema
  import Ecto.Changeset

  # Disable autogeneration for id if necessary, but SERIAL implies db specific. Ecto handles it.
  # timestamps() uses inserted_at/updated_at by default.
  # The schema has `created_at` and `last_login`. It does NOT have `updated_at`.
  # So I should map fields manually and disable detailed timestamps.

  schema "users" do
    field :username, :string
    field :email, :string
    field :created_at, :naive_datetime
    field :last_login, :naive_datetime
    field :settings, :map

    has_many :posts, WfbPhoenix.Models.Post

    # No standard timestamps
  end

  def changeset(user, attrs) do
    user
    |> cast(attrs, [:last_login])
  end
end
