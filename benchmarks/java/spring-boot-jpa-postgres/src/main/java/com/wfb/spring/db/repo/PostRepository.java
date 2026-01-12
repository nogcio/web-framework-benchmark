package com.wfb.spring.db.repo;

import com.wfb.spring.db.entity.PostEntity;
import org.springframework.data.jpa.repository.JpaRepository;
import java.util.List;

public interface PostRepository extends JpaRepository<PostEntity, Integer> {
    List<PostEntity> findTop5ByOrderByViewsDesc();
    List<PostEntity> findTop10ByUserIdOrderByCreatedAtDesc(Integer userId);
}
