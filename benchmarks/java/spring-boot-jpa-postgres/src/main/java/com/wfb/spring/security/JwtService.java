package com.wfb.spring.security;

import com.auth0.jwt.JWT;
import com.auth0.jwt.JWTVerifier;
import com.auth0.jwt.algorithms.Algorithm;
import com.auth0.jwt.exceptions.JWTVerificationException;
import com.auth0.jwt.interfaces.DecodedJWT;
import org.springframework.stereotype.Service;

@Service
public class JwtService {
    private final Algorithm algorithm;
    private final JWTVerifier verifier;

    public JwtService() {
        // Secret doesn't need to match other frameworks; only must be self-consistent.
        String secret = System.getenv().getOrDefault("JWT_SECRET", "super_secret_key_for_benchmarking_only_12345");
        this.algorithm = Algorithm.HMAC256(secret);
        this.verifier = JWT.require(algorithm).build();
    }

    public String createToken(int userId, String username) {
        return JWT.create()
                .withClaim("sub", userId)
                .withClaim("name", username)
                .sign(algorithm);
    }

    public JwtPrincipal verify(String token) throws JWTVerificationException {
        DecodedJWT jwt = verifier.verify(token);
        Integer userId = jwt.getClaim("sub").asInt();
        String username = jwt.getClaim("name").asString();
        if (userId == null || username == null) {
            throw new JWTVerificationException("Missing required claims");
        }
        return new JwtPrincipal(userId, username);
    }
}
