package main

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"io"
	"net/http"
	"os"
	"strconv"
	"time"

	"github.com/gin-gonic/gin"
	_ "github.com/goccy/go-json"
	"github.com/golang-jwt/jwt/v5"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

var jwtSecret = []byte("secret")

type Claims struct {
	UserID   int32  `json:"sub"`
	Username string `json:"name"`
	jwt.RegisteredClaims
}

type HelloWorld struct {
	ID        int32     `json:"id"`
	Name      string    `json:"name"`
	CreatedAt time.Time `json:"createdAt"`
	UpdatedAt time.Time `json:"updatedAt"`
}

type User struct {
	ID           int32  `json:"id"`
	Username     string `json:"username"`
	PasswordHash string `json:"-"`
}

type Tweet struct {
	ID        int32     `json:"id"`
	UserID    int32     `json:"-"`
	Username  string    `json:"username,omitempty"`
	Content   string    `json:"content"`
	CreatedAt time.Time `json:"createdAt"`
	Likes     int64     `json:"likes"`
}

var db *pgxpool.Pool

func main() {
	gin.SetMode(gin.ReleaseMode)
	gin.DefaultWriter = io.Discard
	gin.DefaultErrorWriter = io.Discard

	initDB()
	defer db.Close()

	r := gin.New()
	r.Use(gin.Recovery())

	// Middleware for X-Request-ID
	r.Use(func(c *gin.Context) {
		if requestID := c.GetHeader("X-Request-ID"); requestID != "" {
			c.Header("X-Request-ID", requestID)
		}
		c.Next()
	})

	r.GET("/health", func(c *gin.Context) {
		c.String(http.StatusOK, "OK")
	})

	r.GET("/db/read/one", func(c *gin.Context) {
		idStr := c.Query("id")
		id, err := strconv.Atoi(idStr)
		if err != nil {
			c.Status(http.StatusBadRequest)
			return
		}

		var hw HelloWorld
		err = db.QueryRow(context.Background(), "SELECT id, name, created_at, updated_at FROM hello_world WHERE id = $1", id).Scan(&hw.ID, &hw.Name, &hw.CreatedAt, &hw.UpdatedAt)
		if err != nil {
			c.Status(http.StatusInternalServerError)
			return
		}
		c.JSON(http.StatusOK, hw)
	})

	r.GET("/db/read/many", func(c *gin.Context) {
		offsetStr := c.Query("offset")
		limitStr := c.DefaultQuery("limit", "50")

		offset, err := strconv.Atoi(offsetStr)
		if err != nil {
			c.Status(http.StatusBadRequest)
			return
		}
		limit, err := strconv.Atoi(limitStr)
		if err != nil {
			c.Status(http.StatusBadRequest)
			return
		}

		rows, err := db.Query(context.Background(), "SELECT id, name, created_at, updated_at FROM hello_world ORDER BY id LIMIT $1 OFFSET $2", limit, offset)
		if err != nil {
			c.Status(http.StatusInternalServerError)
			return
		}
		defer rows.Close()

		results := make([]HelloWorld, 0)
		for rows.Next() {
			var hw HelloWorld
			if err := rows.Scan(&hw.ID, &hw.Name, &hw.CreatedAt, &hw.UpdatedAt); err != nil {
				c.Status(http.StatusInternalServerError)
				return
			}
			results = append(results, hw)
		}
		if err := rows.Err(); err != nil {
			c.Status(http.StatusInternalServerError)
			return
		}
		c.JSON(http.StatusOK, results)
	})

	r.POST("/db/write/insert", func(c *gin.Context) {
		var input struct {
			Name string `json:"name"`
		}
		if err := c.ShouldBindJSON(&input); err != nil {
			c.Status(http.StatusBadRequest)
			return
		}

		now := time.Now()
		var hw HelloWorld
		err := db.QueryRow(
			context.Background(),
			"INSERT INTO hello_world (name, created_at, updated_at) VALUES ($1, $2, $3) RETURNING id, name, created_at, updated_at",
			input.Name,
			now,
			now,
		).Scan(&hw.ID, &hw.Name, &hw.CreatedAt, &hw.UpdatedAt)
		if err != nil {
			c.Status(http.StatusInternalServerError)
			return
		}

		c.JSON(http.StatusOK, hw)
	})

	// Tweet Service Endpoints
	api := r.Group("/api")
	{
		api.POST("/auth/register", func(c *gin.Context) {
			var input struct {
				Username string `json:"username"`
				Password string `json:"password"`
			}
			if err := c.ShouldBindJSON(&input); err != nil {
				c.Status(http.StatusBadRequest)
				return
			}

			hash := sha256.Sum256([]byte(input.Password))
			passwordHash := hex.EncodeToString(hash[:])

			_, err := db.Exec(context.Background(), "INSERT INTO users (username, password_hash) VALUES ($1, $2)", input.Username, passwordHash)
			if err != nil {
				c.Status(http.StatusInternalServerError)
				return
			}
			c.Status(http.StatusCreated)
		})

		api.POST("/auth/login", func(c *gin.Context) {
			var input struct {
				Username string `json:"username"`
				Password string `json:"password"`
			}
			if err := c.ShouldBindJSON(&input); err != nil {
				c.Status(http.StatusBadRequest)
				return
			}

			hash := sha256.Sum256([]byte(input.Password))
			passwordHash := hex.EncodeToString(hash[:])

			var id int32
			err := db.QueryRow(context.Background(), "SELECT id FROM users WHERE username = $1 AND password_hash = $2", input.Username, passwordHash).Scan(&id)
			if err != nil {
				c.Status(http.StatusUnauthorized)
				return
			}

			claims := Claims{
				UserID:   id,
				Username: input.Username,
				RegisteredClaims: jwt.RegisteredClaims{
					ExpiresAt: jwt.NewNumericDate(time.Now().Add(24 * time.Hour)),
				},
			}

			token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
			tokenString, err := token.SignedString(jwtSecret)
			if err != nil {
				c.Status(http.StatusInternalServerError)
				return
			}

			c.JSON(http.StatusOK, gin.H{"token": tokenString})
		})

		authorized := api.Group("/")
		authorized.Use(authMiddleware())
		{
			authorized.GET("/feed", func(c *gin.Context) {
				rows, err := db.Query(context.Background(), `
					SELECT t.id, t.user_id, u.username, t.content, t.created_at, (SELECT COUNT(*) FROM likes l WHERE l.tweet_id = t.id) as likes
					FROM tweets t
					JOIN users u ON t.user_id = u.id
					ORDER BY t.created_at DESC
					LIMIT 20
				`)
				if err != nil {
					c.Status(http.StatusInternalServerError)
					return
				}
				defer rows.Close()

				tweets := make([]Tweet, 0)
				for rows.Next() {
					var t Tweet
					if err := rows.Scan(&t.ID, &t.UserID, &t.Username, &t.Content, &t.CreatedAt, &t.Likes); err != nil {
						c.Status(http.StatusInternalServerError)
						return
					}
					tweets = append(tweets, t)
				}
				if err := rows.Err(); err != nil {
					c.Status(http.StatusInternalServerError)
					return
				}
				c.JSON(http.StatusOK, tweets)
			})

			authorized.GET("/tweets/:id", func(c *gin.Context) {
				idStr := c.Param("id")
				id, err := strconv.Atoi(idStr)
				if err != nil {
					c.Status(http.StatusBadRequest)
					return
				}

				var t Tweet
				err = db.QueryRow(context.Background(), `
					SELECT t.id, t.user_id, u.username, t.content, t.created_at, (SELECT COUNT(*) FROM likes l WHERE l.tweet_id = t.id) as likes
					FROM tweets t
					JOIN users u ON t.user_id = u.id
					WHERE t.id = $1
				`, id).Scan(&t.ID, &t.UserID, &t.Username, &t.Content, &t.CreatedAt, &t.Likes)
				if err != nil {
					c.Status(http.StatusNotFound)
					return
				}
				c.JSON(http.StatusOK, t)
			})

			authorized.POST("/tweets", func(c *gin.Context) {
				userID := c.MustGet("userID").(int32)

				var input struct {
					Content string `json:"content"`
				}
				if err := c.ShouldBindJSON(&input); err != nil {
					c.Status(http.StatusBadRequest)
					return
				}

				_, err := db.Exec(context.Background(), "INSERT INTO tweets (user_id, content) VALUES ($1, $2)", userID, input.Content)
				if err != nil {
					c.Status(http.StatusInternalServerError)
					return
				}
				c.Status(http.StatusCreated)
			})

			authorized.POST("/tweets/:id/like", func(c *gin.Context) {
				userID := c.MustGet("userID").(int32)

				tweetIDStr := c.Param("id")
				tweetID, err := strconv.Atoi(tweetIDStr)
				if err != nil {
					c.Status(http.StatusBadRequest)
					return
				}

				// Toggle like
				tag, err := db.Exec(context.Background(), "DELETE FROM likes WHERE user_id = $1 AND tweet_id = $2", userID, tweetID)
				if err != nil {
					c.Status(http.StatusInternalServerError)
					return
				}

				if tag.RowsAffected() == 0 {
					_, err = db.Exec(context.Background(), "INSERT INTO likes (user_id, tweet_id) VALUES ($1, $2)", userID, tweetID)
					if err != nil {
						c.Status(http.StatusInternalServerError)
						return
					}
				}

				c.Status(http.StatusOK)
			})
		}
	}

	port := os.Getenv("PORT")
	if port == "" {
		port = "8000"
	}
	r.Run(":" + port)
}

func authMiddleware() gin.HandlerFunc {
	return func(c *gin.Context) {
		authHeader := c.GetHeader("Authorization")
		if len(authHeader) < 7 || authHeader[:7] != "Bearer " {
			c.AbortWithStatus(http.StatusUnauthorized)
			return
		}
		tokenString := authHeader[7:]

		claims := &Claims{}
		token, err := jwt.ParseWithClaims(tokenString, claims, func(token *jwt.Token) (interface{}, error) {
			return jwtSecret, nil
		})

		if err != nil || !token.Valid {
			c.AbortWithStatus(http.StatusUnauthorized)
			return
		}

		c.Set("userID", claims.UserID)
		c.Next()
	}
}

func initDB() {
	host := os.Getenv("DB_HOST")
	if host == "" {
		host = "localhost"
	}
	port := os.Getenv("DB_PORT")
	if port == "" {
		port = "5432"
	}
	user := os.Getenv("DB_USER")
	if user == "" {
		user = "benchmark"
	}
	password := os.Getenv("DB_PASSWORD")
	if password == "" {
		password = "benchmark"
	}
	dbname := os.Getenv("DB_NAME")
	if dbname == "" {
		dbname = "benchmark"
	}

	dsn := fmt.Sprintf("postgres://%s:%s@%s:%s/%s", user, password, host, port, dbname)
	config, err := pgxpool.ParseConfig(dsn)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Unable to parse config: %v\n", err)
		os.Exit(1)
	}
	config.MaxConns = 256
	config.MinConns = 256
	// Cache prepared statements & result descriptions to reduce per-request overhead on hot SQL.
	// This makes db_read_one/db_write more representative of a tuned production setup.
	config.ConnConfig.DefaultQueryExecMode = pgx.QueryExecModeCacheStatement
	config.ConnConfig.StatementCacheCapacity = 512
	config.ConnConfig.DescriptionCacheCapacity = 512

	db, err = pgxpool.NewWithConfig(context.Background(), config)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Unable to connect to database: %v\n", err)
		os.Exit(1)
	}
}
