package com.wfb.spring.api;

import com.wfb.spring.api.dto.AuthRequest;
import com.wfb.spring.api.dto.TokenResponse;
import com.wfb.spring.api.service.AuthService;
import org.springframework.dao.DataIntegrityViolationException;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

@RestController
@RequestMapping("/api/auth")
public class AuthController {
    private final AuthService authService;

    public AuthController(AuthService authService) {
        this.authService = authService;
    }

    @PostMapping("/register")
    public ResponseEntity<Void> register(@RequestBody AuthRequest req) {
        try {
            authService.register(req);
            return ResponseEntity.status(201).build();
        } catch (DataIntegrityViolationException e) {
            return ResponseEntity.status(409).build();
        }
    }

    @PostMapping("/login")
    public ResponseEntity<TokenResponse> login(@RequestBody AuthRequest req) {
        String token = authService.login(req);
        if (token == null) {
            return ResponseEntity.status(401).build();
        }
        return ResponseEntity.ok(new TokenResponse(token));
    }
}
