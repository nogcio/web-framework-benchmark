package com.wfb.spring.api.dto;

import com.fasterxml.jackson.annotation.JsonProperty;

import java.time.Instant;

public record TweetDto(
        int id,
        String username,
        String content,
        @JsonProperty("createdAt") Instant createdAt,
        long likes
) {
}
