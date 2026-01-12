package com.wfb.spring.api.dto;

public record PostDto(
    Integer id,
    String title,
    String content,
    Integer views,
    String createdAt
) {}
