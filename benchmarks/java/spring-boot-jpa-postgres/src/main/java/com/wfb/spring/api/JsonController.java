package com.wfb.spring.api;

import com.wfb.spring.api.dto.AggregateResponse;
import com.wfb.spring.api.dto.Order;
import com.wfb.spring.api.dto.OrderItem;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RestController;

import java.util.HashMap;
import java.util.List;
import java.util.Map;

@RestController
public class JsonController {

    @PostMapping("/json/aggregate")
    public AggregateResponse aggregate(@RequestBody List<Order> orders) {
        int processedOrders = 0;
        Map<String, Long> results = new HashMap<>();
        Map<String, Integer> categoryStats = new HashMap<>();

        for (Order order : orders) {
            if ("completed".equals(order.getStatus())) {
                processedOrders++;

                results.merge(order.getCountry(), order.getAmount(), Long::sum);

                if (order.getItems() != null) {
                    for (OrderItem item : order.getItems()) {
                        categoryStats.merge(item.getCategory(), item.getQuantity(), Integer::sum);
                    }
                }
            }
        }

        return new AggregateResponse(processedOrders, results, categoryStats);
    }
}
