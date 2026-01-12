package org.acme.model;

import java.util.Map;

public class AggregateResponse {
    public int processedOrders;
    public Map<String, Long> results;
    public Map<String, Integer> categoryStats;

    public AggregateResponse(int processedOrders, Map<String, Long> results, Map<String, Integer> categoryStats) {
        this.processedOrders = processedOrders;
        this.results = results;
        this.categoryStats = categoryStats;
    }
}
