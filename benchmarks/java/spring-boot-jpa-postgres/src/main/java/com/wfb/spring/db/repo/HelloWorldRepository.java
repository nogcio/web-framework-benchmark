package com.wfb.spring.db.repo;

import com.wfb.spring.db.entity.HelloWorldEntity;
import org.springframework.data.domain.Pageable;
import org.springframework.data.jpa.repository.JpaRepository;

import java.util.List;

public interface HelloWorldRepository extends JpaRepository<HelloWorldEntity, Integer> {
    List<HelloWorldEntity> findByOrderByIdAsc(Pageable pageable);
}
