package main

import (
"context"
"fmt"
"io"
"net/http"
"os"
"time"

"github.com/gin-gonic/gin"
"github.com/goccy/go-json"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

type User struct {
	ID        int32           `json:"-"`
	Username  string          `json:"username"`
	Email     string          `json:"email"`
	CreatedAt time.Time       `json:"createdAt"`
	LastLogin *time.Time      `json:"lastLogin"`
	Settings  json.RawMessage `json:"settings"`
}

type Post struct {
	ID        int32     `json:"id"`
	Title     string    `json:"title"`
	Content   string    `json:"content"`
	Views     int32     `json:"views"`
	CreatedAt time.Time `json:"createdAt"`
}

type UserProfile struct {
	Username  string          `json:"username"`
	Email     string          `json:"email"`
	CreatedAt time.Time       `json:"createdAt"`
	LastLogin *time.Time      `json:"lastLogin"`
	Settings  json.RawMessage `json:"settings"`
	Posts     []Post          `json:"posts"`
	Trending  []Post          `json:"trending"`
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

	r.GET("/health", func(c *gin.Context) {
		if err := db.Ping(context.Background()); err != nil {
			c.Status(http.StatusInternalServerError)
			return
		}
		c.String(http.StatusOK, "OK")
	})

	r.GET("/db/user-profile/:email", func(c *gin.Context) {
		email := c.Param("email")
		handleUserProfile(c, email)
	})

	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}
	r.Run(":" + port)
}

func handleUserProfile(c *gin.Context, email string) {
	ctx := context.Background()

	// Parallel Execution: Query A (User) and Query B (Trending)
	type UserResult struct {
		User User
		Err  error
	}
	type TrendingResult struct {
		Posts []Post
		Err   error
	}

	chUser := make(chan UserResult, 1)
	chTrending := make(chan TrendingResult, 1)

	go func() {
		var u User
		err := db.QueryRow(ctx, "SELECT id, username, email, created_at, last_login, settings FROM users WHERE email = $1", email).
			Scan(&u.ID, &u.Username, &u.Email, &u.CreatedAt, &u.LastLogin, &u.Settings)
		chUser <- UserResult{User: u, Err: err}
	}()

	go func() {
		rows, err := db.Query(ctx, "SELECT id, title, content, views, created_at FROM posts ORDER BY views DESC LIMIT 5")
		if err != nil {
			chTrending <- TrendingResult{Err: err}
			return
		}
		defer rows.Close()
		var posts []Post
		for rows.Next() {
			var p Post
			if err := rows.Scan(&p.ID, &p.Title, &p.Content, &p.Views, &p.CreatedAt); err != nil {
				chTrending <- TrendingResult{Err: err}
				return
			}
			posts = append(posts, p)
		}
		chTrending <- TrendingResult{Posts: posts, Err: nil}
	}()

	userRes := <-chUser
	trendingRes := <-chTrending

	if userRes.Err != nil {
		if userRes.Err == pgx.ErrNoRows {
			c.JSON(http.StatusNotFound, gin.H{"error": "User not found"})
		} else {
			c.Status(http.StatusInternalServerError)
		}
		return
	}

	if trendingRes.Err != nil {
		c.Status(http.StatusInternalServerError)
		return
	}

	// Phase 2: Parallel Update and Fetch Posts
	type UpdateResult struct {
		Err error
	}
	type PostsResult struct {
		Posts []Post
		Err   error
	}

	chUpdate := make(chan UpdateResult, 1)
	chPosts := make(chan PostsResult, 1)

	// Task C: Update Last Login
	go func() {
		var newLastLogin time.Time
		err := db.QueryRow(ctx, "UPDATE users SET last_login = NOW() WHERE id = $1 RETURNING last_login", userRes.User.ID).Scan(&newLastLogin)
		if err == nil {
			userRes.User.LastLogin = &newLastLogin
		}
		chUpdate <- UpdateResult{Err: err}
	}()

	// Task D: User Posts
	go func() {
		rows, err := db.Query(ctx, "SELECT id, title, content, views, created_at FROM posts WHERE user_id = $1 ORDER BY created_at DESC LIMIT 10", userRes.User.ID)
		if err != nil {
			chPosts <- PostsResult{Err: err}
			return
		}
		defer rows.Close()

		var userPosts []Post
		for rows.Next() {
			var p Post
			if err := rows.Scan(&p.ID, &p.Title, &p.Content, &p.Views, &p.CreatedAt); err != nil {
				chPosts <- PostsResult{Err: err}
				return
			}
			userPosts = append(userPosts, p)
		}
		chPosts <- PostsResult{Posts: userPosts, Err: nil}
	}()

	updateRes := <-chUpdate
	postsRes := <-chPosts

	if updateRes.Err != nil {
		c.Status(http.StatusInternalServerError)
		return
	}
	if postsRes.Err != nil {
		c.Status(http.StatusInternalServerError)
		return
	}
	
	userPosts := postsRes.Posts
	// Ensure empty slices are serialized as [] instead of null
	if userPosts == nil {
		userPosts = []Post{}
	}
	if trendingRes.Posts == nil {
		trendingRes.Posts = []Post{}
	}

	profile := UserProfile{
		Username:  userRes.User.Username,
		Email:     userRes.User.Email,
		CreatedAt: userRes.User.CreatedAt,
		LastLogin: userRes.User.LastLogin,
		Settings:  userRes.User.Settings,
		Posts:     userPosts,
		Trending:  trendingRes.Posts,
	}

	c.JSON(http.StatusOK, profile)
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
	
	poolSize := 256
	if ps := os.Getenv("DB_POOL_SIZE"); ps != "" {
		var p int
		if _, err := fmt.Sscanf(ps, "%d", &p); err == nil {
			poolSize = p
		}
	}
	
	config.MaxConns = int32(poolSize)
	config.MinConns = int32(poolSize)
	config.ConnConfig.DefaultQueryExecMode = pgx.QueryExecModeCacheStatement
	config.ConnConfig.StatementCacheCapacity = 1024
	config.ConnConfig.DescriptionCacheCapacity = 1024

	db, err = pgxpool.NewWithConfig(context.Background(), config)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Unable to connect to database: %v\n", err)
		os.Exit(1)
	}
}
