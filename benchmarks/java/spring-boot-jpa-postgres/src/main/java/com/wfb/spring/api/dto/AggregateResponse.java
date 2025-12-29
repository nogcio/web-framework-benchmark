package com.wfb.spring.api.dto;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Map;

public class AggregateResponse {
    @JsonProperty("processedOrders")
    private int processedOrders;
    private Map<String, Long> results;
    @JsonProperty("categoryStats")
    private Map<String, Integer> categoryStats;

    public AggregateResponse(int processedOrders, Map<String, Long> results, Map<String, Integer> categoryStats) {
        this.processedOrders = processedOrders;
        this.results = results;
        this.categoryStats = categoryStats;
    }

    public int getProcessedOrders() {
        return processedOrders;
    }

    public Map<String, Long> getResults() {
        return results;
    }

    public Map<String, Integer> getCategoryStats() {
        return categoryStats;
    }
}
