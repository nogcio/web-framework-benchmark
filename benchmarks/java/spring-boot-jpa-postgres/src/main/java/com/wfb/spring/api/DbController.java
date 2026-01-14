package com.wfb.spring.api;

import com.wfb.spring.api.dto.PostDto;
import com.wfb.spring.api.dto.UserProfileDto;
import com.wfb.spring.db.entity.PostEntity;
import com.wfb.spring.db.entity.UserEntity;
import com.wfb.spring.db.repo.PostRepository;
import com.wfb.spring.db.repo.UserRepository;
import org.springframework.http.HttpStatus;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;
import org.springframework.web.server.ResponseStatusException;

import javax.sql.DataSource;
import java.sql.Connection;
import java.sql.Statement;
import java.time.Instant;
import java.time.ZoneId;
import java.time.format.DateTimeFormatter;
import java.util.List;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.Executor;
import java.util.concurrent.Executors;

@RestController
@RequestMapping("/db")
public class DbController {
    private final UserRepository userRepository;
    private final PostRepository postRepository;
    private final DataSource dataSource;
    private static final DateTimeFormatter ISO_FORMATTER = DateTimeFormatter.ISO_INSTANT.withZone(ZoneId.of("UTC"));
    private static final Executor VT_EXECUTOR = Executors.newVirtualThreadPerTaskExecutor();

    public DbController(UserRepository userRepository, PostRepository postRepository, DataSource dataSource) {
        this.userRepository = userRepository;
        this.postRepository = postRepository;
        this.dataSource = dataSource;
    }

    @GetMapping("/user-profile/{email}")
    public UserProfileDto getUserProfile(@PathVariable String email) {
        CompletableFuture<UserEntity> userFuture = CompletableFuture.supplyAsync(() -> 
            userRepository.findByEmail(email).orElse(null), VT_EXECUTOR
        );
        CompletableFuture<List<PostEntity>> trendingFuture = CompletableFuture.supplyAsync(() -> 
            postRepository.findTop5ByOrderByViewsDesc(), VT_EXECUTOR
        );

        CompletableFuture.allOf(userFuture, trendingFuture).join();

        UserEntity user = userFuture.join();
        if (user == null) {
            throw new ResponseStatusException(HttpStatus.NOT_FOUND, "User not found");
        }

        CompletableFuture<Void> updateFuture = CompletableFuture.runAsync(() -> {
            user.setLastLogin(Instant.now());
            userRepository.save(user);
        }, VT_EXECUTOR);

        CompletableFuture<List<PostEntity>> postsFuture = CompletableFuture.supplyAsync(() -> 
            postRepository.findTop10ByUserIdOrderByCreatedAtDesc(user.getId()), VT_EXECUTOR
        );

        CompletableFuture.allOf(updateFuture, postsFuture).join();

        return mapToDto(user, postsFuture.join(), trendingFuture.join());
    }

    private UserProfileDto mapToDto(UserEntity user, List<PostEntity> posts, List<PostEntity> trending) {
        return new UserProfileDto(
            user.getUsername(),
            user.getEmail(),
            user.getCreatedAt() != null ? ISO_FORMATTER.format(user.getCreatedAt()) : null,
            user.getLastLogin() != null ? ISO_FORMATTER.format(user.getLastLogin()) : null,
            user.getSettings(),
            posts.stream().map(this::mapPost).toList(),
            trending.stream().map(this::mapPost).toList()
        );
    }

    private PostDto mapPost(PostEntity p) {
        return new PostDto(
            p.getId(),
            p.getTitle(),
            p.getContent(),
            p.getViews(),
            p.getCreatedAt() != null ? ISO_FORMATTER.format(p.getCreatedAt()) : null
        );
    }
    
    @GetMapping("/health")
    public ResponseEntity<String> health() {
        try (Connection conn = dataSource.getConnection(); Statement stmt = conn.createStatement()) {
            stmt.execute("SELECT 1");
            return ResponseEntity.ok("OK");
        } catch (Exception e) {
            return ResponseEntity.status(500).body("Database unavailable");
        }
    }
}
