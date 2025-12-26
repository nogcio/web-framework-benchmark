package com.wfb.spring.api.service;

import com.wfb.spring.api.dto.AuthRequest;
import com.wfb.spring.db.entity.UserEntity;
import com.wfb.spring.db.repo.UserRepository;
import com.wfb.spring.security.JwtService;
import com.wfb.spring.util.Sha256;
import org.springframework.dao.DataIntegrityViolationException;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

@Service
public class AuthService {
    private final UserRepository userRepository;
    private final JwtService jwtService;

    public AuthService(UserRepository userRepository, JwtService jwtService) {
        this.userRepository = userRepository;
        this.jwtService = jwtService;
    }

    @Transactional
    public void register(AuthRequest req) {
        UserEntity user = new UserEntity();
        user.setUsername(req.username());
        user.setPasswordHash(Sha256.encode(req.password()));
        try {
            userRepository.save(user);
        } catch (DataIntegrityViolationException e) {
            throw e;
        }
    }

    @Transactional(readOnly = true)
    public String login(AuthRequest req) {
        UserEntity user = userRepository.findByUsername(req.username()).orElse(null);
        if (user == null) {
            return null;
        }

        String hash = Sha256.encode(req.password());
        if (!hash.equals(user.getPasswordHash())) {
            return null;
        }

        return jwtService.createToken(user.getId(), user.getUsername());
    }
}
