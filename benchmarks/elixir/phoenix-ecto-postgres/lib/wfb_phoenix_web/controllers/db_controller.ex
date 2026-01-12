defmodule WfbPhoenixWeb.DbController do
  use WfbPhoenixWeb, :controller
  import Ecto.Query
  alias WfbPhoenix.Repo
  alias WfbPhoenix.Models.{User, Post}

  def user_profile(conn, %{"email" => email}) do
    # Parallelize initial independent queries
    user_task = Task.async(fn -> 
      Repo.get_by(User, email: email)
    end)

    trending_task = Task.async(fn -> 
      query = from p in Post,
        order_by: [desc: p.views],
        limit: 5,
        select: map(p, [:id, :title, :content, :views, :created_at]) # Select specific fields for JSON
      
      Repo.all(query)
    end)

    # Await User
    user = Task.await(user_task)

    if user do
      trending = Task.await(trending_task)
      
      # Determine current time once
      now = NaiveDateTime.truncate(NaiveDateTime.utc_now(), :second)

      # Parallelize dependent operations
      # 1. Update last_login
      update_task = Task.async(fn ->
        user
        |> User.changeset(%{last_login: now})
        |> Repo.update!()
      end)

      # 2. Fetch user posts
      posts_task = Task.async(fn ->
        query = from p in Post,
          where: p.user_id == ^user.id,
          order_by: [desc: p.created_at],
          limit: 10,
          select: map(p, [:id, :title, :content, :views, :created_at])

        Repo.all(query)
      end)

      updated_user = Task.await(update_task)
      posts = Task.await(posts_task)

      # Format Response
      response = %{
        username: updated_user.username,
        email: updated_user.email,
        createdAt: updated_user.created_at,
        lastLogin: updated_user.last_login,
        settings: updated_user.settings,
        posts: format_posts(posts),
        trending: format_posts(trending)
      }

      json(conn, response)
    else
      # Ensure trending query handles even if user not found (to avoid leaking processes, though Task.await/2 handles it or Task.shutdown)
      # Actually Task.await(trending_task) needs to be called to avoid leaving the process hanging or crash? 
      # `Task.async` links the process. If we crash/exit, it dies.
      # But best practice to await or ignore.
      Task.await(trending_task) 
      
      send_resp(conn, 404, "User Not Found")
    end
  end

  defp format_posts(posts) do
    Enum.map(posts, fn p -> 
      %{
        id: p.id,
        title: p.title,
        content: p.content,
        views: p.views,
        createdAt: p.created_at
      }
    end)
  end
end
