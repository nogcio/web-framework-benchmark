package com.wfb.spring.api.service;

import com.wfb.spring.api.dto.CreateTweetRequest;
import com.wfb.spring.api.dto.TweetDto;
import org.springframework.dao.DataIntegrityViolationException;
import com.wfb.spring.db.entity.LikeEntity;
import com.wfb.spring.db.entity.LikeId;
import com.wfb.spring.db.entity.TweetEntity;
import com.wfb.spring.db.entity.UserEntity;
import com.wfb.spring.db.repo.LikeRepository;
import com.wfb.spring.db.repo.TweetRepository;
import com.wfb.spring.db.repo.UserRepository;
import org.springframework.data.domain.PageRequest;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.time.Instant;
import java.util.List;
import java.util.Optional;

@Service
public class TweetService {
    private final TweetRepository tweetRepository;
    private final LikeRepository likeRepository;
    private final UserRepository userRepository;

    public TweetService(TweetRepository tweetRepository, LikeRepository likeRepository, UserRepository userRepository) {
        this.tweetRepository = tweetRepository;
        this.likeRepository = likeRepository;
        this.userRepository = userRepository;
    }

    @Transactional(readOnly = true)
    public List<TweetDto> feed() {
        return tweetRepository.feed(PageRequest.of(0, 20));
    }

    @Transactional(readOnly = true)
    public Optional<TweetDto> get(int id) {
        return tweetRepository.getTweet(id);
    }

    @Transactional
    public Optional<TweetDto> createTweet(int userId, CreateTweetRequest req) {
        UserEntity user = userRepository.findById(userId).orElse(null);
        if (user == null) {
            return Optional.empty();
        }

        TweetEntity tweet = new TweetEntity();
        tweet.setUser(user);
        tweet.setContent(req.content());
        tweet.setCreatedAt(Instant.now());
        TweetEntity saved = tweetRepository.save(tweet);
        return Optional.of(new TweetDto(
                saved.getId(),
                user.getUsername(),
                saved.getContent(),
                saved.getCreatedAt(),
                0
        ));
    }

    @Transactional
    public ToggleResult toggleLike(int userId, int tweetId) {
        // Match aspnetcore-efcore flow:
        // 1) ensure tweet exists
        // 2) find existing like by (user_id,tweet_id) and toggle
        if (!tweetRepository.existsById(tweetId)) {
            return ToggleResult.NOT_FOUND;
        }

        LikeId id = new LikeId(userId, tweetId);
        try {
            if (likeRepository.existsById(id)) {
                likeRepository.deleteById(id);
            } else {
                // Ensure user exists for FK integrity
                UserEntity user = userRepository.findById(userId).orElse(null);
                TweetEntity tweet = tweetRepository.findById(tweetId).orElse(null);
                if (user == null || tweet == null) {
                    return ToggleResult.CONFLICT;
                }
                likeRepository.save(new LikeEntity(user, tweet));
            }
            return ToggleResult.OK;
        } catch (DataIntegrityViolationException e) {
            return ToggleResult.CONFLICT;
        }
    }

    public enum ToggleResult {
        OK,
        NOT_FOUND,
        CONFLICT
    }
}
