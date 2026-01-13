using System.Runtime.InteropServices;
using Grpc.Core;
using Wfb.AspNetCore.Grpc;

namespace Wfb.AspNetCore.Grpc.Services;

public class AnalyticsService : Wfb.AspNetCore.Grpc.AnalyticsService.AnalyticsServiceBase
{
    public override Task<AggregateResult> AggregateOrders(AnalyticsRequest request, ServerCallContext context)
    {
        int processedOrders = 0;
        var amountByCountry = new Dictionary<string, long>(4);
        var quantityByCategory = new Dictionary<string, int>(4);
        
        // Read client id from metadata
        // "x-client-id" keys in metadata are lowercased
        var clientId = context.RequestHeaders.GetValue("x-client-id") ?? "";

        foreach (var order in request.Orders)
        {
            if (order.Status == OrderStatus.Completed)
            {
                processedOrders++;
                
                long orderAmount = 0;
                foreach (var item in order.Items)
                {
                    long itemTotal = item.PriceCents * item.Quantity;
                    orderAmount += itemTotal;

                    ref int qty = ref CollectionsMarshal.GetValueRefOrAddDefault(quantityByCategory, item.Category, out _);
                    qty += item.Quantity;
                }

                ref long amt = ref CollectionsMarshal.GetValueRefOrAddDefault(amountByCountry, order.Country, out _);
                amt += orderAmount;
            }
        }

        var result = new AggregateResult
        {
            ProcessedOrders = processedOrders,
            EchoedClientId = clientId
        };
        
        // Copy dictionaries to protobuf map fields
        result.AmountByCountry.Add(amountByCountry);
        result.QuantityByCategory.Add(quantityByCategory);

        return Task.FromResult(result);
    }
}
