defmodule AnalyticsService.Server do
  use GRPC.Server, service: AnalyticsService.Service

  @spec aggregate_orders(AnalyticsRequest.t, GRPC.Server.Stream.t) :: AggregateResult.t
  def aggregate_orders(request, stream) do
    # Only process COMPLETED orders
    completed_orders = Enum.filter(request.orders, fn o -> o.status == :COMPLETED end)
    processed_orders = length(completed_orders)

    headers = GRPC.Stream.get_headers(stream)
    client_id = Map.get(headers, "x-client-id", "")
    
    {amount_by_country, quantity_by_category} = 
      Enum.reduce(completed_orders, {%{}, %{}}, fn order, {acc_amount, acc_qty} ->
        # Calculate amount for this order
        order_amount = 
          Enum.reduce(order.items, 0, fn item, acc -> 
            acc + (item.price_cents * item.quantity) 
          end)
        
        # Update amount by country
        new_acc_amount = Map.update(acc_amount, order.country, order_amount, &(&1 + order_amount))
        
        # Update quantity by category
        new_acc_qty = 
          Enum.reduce(order.items, acc_qty, fn item, inner_acc_qty ->
            Map.update(inner_acc_qty, item.category, item.quantity, &(&1 + item.quantity))
          end)
          
        {new_acc_amount, new_acc_qty}
      end)

    AggregateResult.new(
      processed_orders: processed_orders,
      amount_by_country: amount_by_country,
      quantity_by_category: quantity_by_category,
      echoed_client_id: client_id
    )
  end
end
