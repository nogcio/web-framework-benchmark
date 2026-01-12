package com.wfb.spring.api.dto;

import java.util.List;

public record UserProfileDto(
    String username,
    String email,
    String createdAt,
    String lastLogin,
    Object settings,
    List<PostDto> posts,
    List<PostDto> trending
) {}
