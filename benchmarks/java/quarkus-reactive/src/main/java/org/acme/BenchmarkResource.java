package org.acme;

import jakarta.ws.rs.GET;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.MediaType;
import org.acme.model.AggregateResponse;
import org.acme.model.Order;
import org.acme.model.OrderItem;

import java.util.HashMap;
import java.util.List;
import java.util.Map;

@Path("/")
public class BenchmarkResource {

    @GET
    @Path("/plaintext")
    @Produces(MediaType.TEXT_PLAIN)
    public String plaintext() {
        return "Hello, World!";
    }

    @GET
    @Path("/health")
    public String health() {
        return "OK";
    }

    @POST
    @Path("/json/aggregate")
    @Produces(MediaType.APPLICATION_JSON)
    public AggregateResponse jsonAggregate(List<Order> orders) {
        int processedOrders = 0;
        Map<String, Long> results = new HashMap<>();
        Map<String, Integer> categoryStats = new HashMap<>();

        if (orders != null) {
            for (Order order : orders) {
                if ("completed".equals(order.status)) {
                    processedOrders++;
                    
                    results.merge(order.country, order.amount, Long::sum);

                    if (order.items != null) {
                        for (OrderItem item : order.items) {
                            categoryStats.merge(item.category, item.quantity, Integer::sum);
                        }
                    }
                }
            }
        }

        return new AggregateResponse(processedOrders, results, categoryStats);
    }
}
