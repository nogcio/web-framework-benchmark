package com.wfb.spring.api;

import com.wfb.spring.api.dto.CreateTweetRequest;
import com.wfb.spring.api.dto.TweetDto;
import com.wfb.spring.api.service.TweetService;
import com.wfb.spring.security.JwtPrincipal;
import org.springframework.http.ResponseEntity;
import org.springframework.security.core.annotation.AuthenticationPrincipal;
import org.springframework.web.bind.annotation.*;

import java.util.List;

@RestController
@RequestMapping("/api")
public class TweetController {
    private final TweetService tweetService;

    public TweetController(TweetService tweetService) {
        this.tweetService = tweetService;
    }

    @GetMapping("/feed")
    public ResponseEntity<List<TweetDto>> feed() {
        return ResponseEntity.ok(tweetService.feed());
    }

    @GetMapping("/tweets/{id}")
    public ResponseEntity<TweetDto> getTweet(@PathVariable int id) {
        return tweetService.get(id)
                .map(ResponseEntity::ok)
                .orElseGet(() -> ResponseEntity.notFound().build());
    }

    @PostMapping("/tweets")
    public ResponseEntity<TweetDto> createTweet(
            @AuthenticationPrincipal JwtPrincipal principal,
            @RequestBody CreateTweetRequest req
    ) {
        return tweetService.createTweet(principal.userId(), req)
                .map(dto -> ResponseEntity.status(201).body(dto))
                .orElseGet(() -> ResponseEntity.status(409).build());
    }

    @PostMapping("/tweets/{id}/like")
    public ResponseEntity<Void> like(
            @AuthenticationPrincipal JwtPrincipal principal,
            @PathVariable int id
    ) {
        TweetService.ToggleResult result = tweetService.toggleLike(principal.userId(), id);
        return switch (result) {
            case OK -> ResponseEntity.ok().build();
            case NOT_FOUND -> ResponseEntity.notFound().build();
            case CONFLICT -> ResponseEntity.status(409).build();
        };
    }
}
