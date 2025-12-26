package com.wfb.spring.api.dto;

import com.fasterxml.jackson.annotation.JsonProperty;

import java.time.Instant;

public record HelloWorldDto(
        int id,
        String name,
        @JsonProperty("createdAt") Instant createdAt,
        @JsonProperty("updatedAt") Instant updatedAt
) {
}
