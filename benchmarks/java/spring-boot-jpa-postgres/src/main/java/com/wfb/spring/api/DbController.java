package com.wfb.spring.api;

import com.wfb.spring.api.dto.HelloWorldDto;
import com.wfb.spring.api.dto.WriteRequest;
import com.wfb.spring.db.entity.HelloWorldEntity;
import com.wfb.spring.db.repo.HelloWorldRepository;
import jakarta.persistence.EntityManager;
import jakarta.persistence.PersistenceContext;
import jakarta.transaction.Transactional;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;

import java.time.Instant;
import java.util.List;

@RestController
@RequestMapping("/db")
public class DbController {
    private final HelloWorldRepository helloWorldRepository;

    @PersistenceContext
    private EntityManager entityManager;

    public DbController(HelloWorldRepository helloWorldRepository) {
        this.helloWorldRepository = helloWorldRepository;
    }

    @GetMapping("/read/one")
    public ResponseEntity<HelloWorldDto> readOne(@RequestParam int id) {
        return helloWorldRepository.findById(id)
                .map(DbController::toDto)
                .map(ResponseEntity::ok)
                .orElseGet(() -> ResponseEntity.notFound().build());
    }

    @GetMapping("/read/many")
    public List<HelloWorldDto> readMany(@RequestParam int offset, @RequestParam(required = false) Integer limit) {
        int actualLimit = (limit == null) ? 50 : limit;

        @SuppressWarnings("unchecked")
        List<HelloWorldEntity> rows = entityManager
                .createQuery("select h from HelloWorldEntity h order by h.id")
                .setFirstResult(offset)
                .setMaxResults(actualLimit)
                .getResultList();

        return rows.stream().map(DbController::toDto).toList();
    }

    @PostMapping("/write/insert")
    @Transactional
    public ResponseEntity<HelloWorldDto> insert(
            @RequestParam(required = false) String name,
            @RequestBody(required = false) WriteRequest body
    ) {
        String resolvedName = name;
        if ((resolvedName == null || resolvedName.isEmpty()) && body != null) {
            resolvedName = body.name();
        }
        if (resolvedName == null || resolvedName.isEmpty()) {
            return ResponseEntity.badRequest().build();
        }

        Instant now = Instant.now();
        HelloWorldEntity entity = new HelloWorldEntity();
        entity.setName(resolvedName);
        entity.setCreatedAt(now);
        entity.setUpdatedAt(now);
        HelloWorldEntity saved = helloWorldRepository.save(entity);

        return ResponseEntity.ok(toDto(saved));
    }

    private static HelloWorldDto toDto(HelloWorldEntity e) {
        return new HelloWorldDto(e.getId(), e.getName(), e.getCreatedAt(), e.getUpdatedAt());
    }
}
