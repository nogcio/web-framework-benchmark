package org.acme.grpc;

import io.grpc.stub.StreamObserver;
import io.quarkus.grpc.GrpcService;
import io.smallrye.common.annotation.Blocking;
import java.util.HashMap;
import java.util.Map;

@GrpcService
@Blocking
public class AnalyticsServiceImpl extends AnalyticsServiceGrpc.AnalyticsServiceImplBase {

    @Override
    public void aggregateOrders(AnalyticsRequest request, StreamObserver<AggregateResult> responseObserver) {
        String clientId = HeaderInterceptor.CLIENT_ID_CTX_KEY.get();
        final String effectiveClientId = clientId == null ? "" : clientId;

        int processedOrders = 0;
        // Using mutable containers to reduce object allocation during map updates
        // Initial capacity 8 is sufficient for benchmark (4 countries, 4 categories) but allows growth
        final Map<String, MutableLong> amountByCountry = new HashMap<>(8);
        final Map<String, MutableInt> quantityByCategory = new HashMap<>(8);

        for (Order order : request.getOrdersList()) {
            if (order.getStatus() == OrderStatus.COMPLETED) {
                processedOrders++;
                
                long orderAmount = 0;
                for (OrderItem item : order.getItemsList()) {
                    long itemTotal = item.getPriceCents() * item.getQuantity();
                    orderAmount += itemTotal;

                    // Aggregation: quantity by category
                    MutableInt qty = quantityByCategory.get(item.getCategory());
                    if (qty == null) {
                        qty = new MutableInt();
                        quantityByCategory.put(item.getCategory(), qty);
                    }
                    qty.val += item.getQuantity();
                }

                // Aggregation: amount by country
                MutableLong amt = amountByCountry.get(order.getCountry());
                if (amt == null) {
                    amt = new MutableLong();
                    amountByCountry.put(order.getCountry(), amt);
                }
                amt.val += orderAmount;
            }
        }

        AggregateResult.Builder builder = AggregateResult.newBuilder()
                .setProcessedOrders(processedOrders)
                .setEchoedClientId(effectiveClientId);

        // Transfer mutable map data to builder
        for (Map.Entry<String, MutableLong> entry : amountByCountry.entrySet()) {
            builder.putAmountByCountry(entry.getKey(), entry.getValue().val);
        }
        for (Map.Entry<String, MutableInt> entry : quantityByCategory.entrySet()) {
            builder.putQuantityByCategory(entry.getKey(), entry.getValue().val);
        }

        responseObserver.onNext(builder.build());
        responseObserver.onCompleted();
    }

    // Lightweight mutable wrappers to avoid Integer/Long object churn
    private static class MutableInt { int val; }
    private static class MutableLong { long val; }
}

