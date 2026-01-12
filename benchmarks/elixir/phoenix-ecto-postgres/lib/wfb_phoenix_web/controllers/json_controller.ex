defmodule WfbPhoenixWeb.JsonController do
  use WfbPhoenixWeb, :controller

  def aggregate(conn, %{"_json" => orders}) when is_list(orders) do
    do_aggregate(conn, orders)
  end

  def aggregate(conn, orders) when is_list(orders) do
    do_aggregate(conn, orders)
  end

  # Fallback for invalid input
  def aggregate(conn, _), do: send_resp(conn, 400, "Bad Request")

  defp do_aggregate(conn, orders) do
    initial_acc = %{
      processedOrders: 0,
      results: %{},
      categoryStats: %{}
    }

    result = Enum.reduce(orders, initial_acc, fn order, acc ->
      if order["status"] == "completed" do
        process_order(order, acc)
      else
        acc
      end
    end)

    json(conn, result)
  end

  defp process_order(order, acc) do
    # Update count
    acc = Map.update!(acc, :processedOrders, &(&1 + 1))

    # Update country results
    country = order["country"]
    amount = order["amount"]
    acc = put_in(acc, [:results, country], Map.get(acc.results, country, 0) + amount)

    # Update category stats
    items = order["items"] || []
    category_stats = Enum.reduce(items, acc.categoryStats, fn item, cat_acc ->
      category = item["category"]
      quantity = item["quantity"]
      Map.update(cat_acc, category, quantity, &(&1 + quantity))
    end)
    
    Map.put(acc, :categoryStats, category_stats)
  end
end
