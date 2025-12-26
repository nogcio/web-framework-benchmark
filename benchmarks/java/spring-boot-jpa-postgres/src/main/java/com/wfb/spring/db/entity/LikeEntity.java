package com.wfb.spring.db.entity;

import jakarta.persistence.*;

@Entity
@Table(name = "likes")
public class LikeEntity {
    @EmbeddedId
    private LikeId id;

    @ManyToOne(fetch = FetchType.LAZY, optional = false)
    @MapsId("userId")
    @JoinColumn(name = "user_id", nullable = false)
    private UserEntity user;

    @ManyToOne(fetch = FetchType.LAZY, optional = false)
    @MapsId("tweetId")
    @JoinColumn(name = "tweet_id", nullable = false)
    private TweetEntity tweet;

    public LikeEntity() {
    }

    public LikeEntity(UserEntity user, TweetEntity tweet) {
        this.user = user;
        this.tweet = tweet;
        this.id = new LikeId(user.getId(), tweet.getId());
    }

    public LikeId getId() {
        return id;
    }

    public UserEntity getUser() {
        return user;
    }

    public TweetEntity getTweet() {
        return tweet;
    }
}
