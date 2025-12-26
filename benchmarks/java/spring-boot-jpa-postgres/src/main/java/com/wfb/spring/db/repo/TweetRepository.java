package com.wfb.spring.db.repo;

import com.wfb.spring.api.dto.TweetDto;
import com.wfb.spring.db.entity.TweetEntity;
import org.springframework.data.domain.Pageable;
import org.springframework.data.jpa.repository.JpaRepository;
import org.springframework.data.jpa.repository.Query;
import org.springframework.data.repository.query.Param;

import java.util.List;
import java.util.Optional;

public interface TweetRepository extends JpaRepository<TweetEntity, Integer> {
		@Query("""
						select new com.wfb.spring.api.dto.TweetDto(
							t.id,
							u.username,
							t.content,
							t.createdAt,
							count(l.user)
						)
						from TweetEntity t
						join t.user u
						left join t.likes l
						group by t.id, u.username, t.content, t.createdAt
						order by t.createdAt desc
						""")
		List<TweetDto> feed(Pageable pageable);

		@Query("""
						select new com.wfb.spring.api.dto.TweetDto(
							t.id,
							u.username,
							t.content,
							t.createdAt,
							count(l.user)
						)
						from TweetEntity t
						join t.user u
						left join t.likes l
						where t.id = :id
						group by t.id, u.username, t.content, t.createdAt
						""")
		Optional<TweetDto> getTweet(@Param("id") int id);
}
