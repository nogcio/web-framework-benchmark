package org.acme.grpc

import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.collect
import java.util.concurrent.ConcurrentHashMap

class AnalyticsService : AnalyticsServiceGrpcKt.AnalyticsServiceCoroutineImplBase() {
    override suspend fun aggregateOrders(request: AnalyticsRequest): AggregateResult {
        val clientId = ClientIdInterceptor.CLIENT_ID_CTX_KEY.get() ?: ""

        var processedOrders = 0
        val amountByCountry = mutableMapOf<String, Long>()
        val quantityByCategory = mutableMapOf<String, Int>()

        for (order in request.ordersList) {
            if (order.status != OrderStatus.COMPLETED) {
                continue
            }
            processedOrders++
            
            // Should match logic in other implementations roughly
            // Assuming simplified logic: sum prices by country, sum quantities by category
            
            // Calculate order total
            var orderTotal = 0L
            for (item in order.itemsList) {
                 orderTotal += (item.priceCents * item.quantity)
                 val existingQty = quantityByCategory.getOrDefault(item.category, 0)
                 quantityByCategory[item.category] = existingQty + item.quantity
            }

            val existingAmount = amountByCountry.getOrDefault(order.country, 0L)
            amountByCountry[order.country] = existingAmount + orderTotal
        }

        return AggregateResult.newBuilder()
            .setProcessedOrders(processedOrders)
            .putAllAmountByCountry(amountByCountry)
            .putAllQuantityByCategory(quantityByCategory)
            .setEchoedClientId(clientId)
            .build()
    }
}
