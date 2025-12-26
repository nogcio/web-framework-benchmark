package com.wfb.spring.db.entity;

import jakarta.persistence.Column;
import jakarta.persistence.Embeddable;

import java.io.Serializable;
import java.util.Objects;

@Embeddable
public class LikeId implements Serializable {
    @Column(name = "user_id")
    private Integer userId;

    @Column(name = "tweet_id")
    private Integer tweetId;

    public LikeId() {
    }

    public LikeId(Integer userId, Integer tweetId) {
        this.userId = userId;
        this.tweetId = tweetId;
    }

    public Integer getUserId() {
        return userId;
    }

    public Integer getTweetId() {
        return tweetId;
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        LikeId likeId = (LikeId) o;
        return Objects.equals(userId, likeId.userId) && Objects.equals(tweetId, likeId.tweetId);
    }

    @Override
    public int hashCode() {
        return Objects.hash(userId, tweetId);
    }
}
