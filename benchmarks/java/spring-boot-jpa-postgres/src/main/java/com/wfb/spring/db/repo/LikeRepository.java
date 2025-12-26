package com.wfb.spring.db.repo;

import com.wfb.spring.db.entity.LikeEntity;
import com.wfb.spring.db.entity.LikeId;
import org.springframework.data.jpa.repository.JpaRepository;

public interface LikeRepository extends JpaRepository<LikeEntity, LikeId> {
}
