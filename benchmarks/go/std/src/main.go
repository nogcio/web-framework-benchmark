package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"time"

	_ "github.com/lib/pq"
)

// Config holds application configuration
type Config struct {
	DBHost     string
	DBUser     string
	DBPassword string
	DBName     string
	DBPort     string
	Port       string
	DataDir    string
}

func loadConfig() Config {
	getEnv := func(key, fallback string) string {
		if v := os.Getenv(key); v != "" {
			return v
		}
		return fallback
	}

	return Config{
		DBHost:     getEnv("DB_HOST", "db"),
		DBUser:     getEnv("DB_USER", "benchmark"),
		DBPassword: getEnv("DB_PASSWORD", "benchmark"),
		DBName:     getEnv("DB_NAME", "benchmark"),
		DBPort:     getEnv("DB_PORT", "5432"),
		Port:       getEnv("PORT", "8000"),
		DataDir:    getEnv("DATA_DIR", "benchmarks_data"),
	}
}

type App struct {
	db  *sql.DB
	cfg Config
}

func (app *App) initDB() error {
	connStr := fmt.Sprintf("postgres://%s:%s@%s:%s/%s?sslmode=disable",
		app.cfg.DBUser, app.cfg.DBPassword, app.cfg.DBHost, app.cfg.DBPort, app.cfg.DBName)

	var err error
	app.db, err = sql.Open("postgres", connStr)
	if err != nil {
		return fmt.Errorf("failed to open db connection: %w", err)
	}

	// Production tuning for connection pool
	app.db.SetMaxOpenConns(128)
	app.db.SetMaxIdleConns(128)
	app.db.SetConnMaxIdleTime(5 * time.Minute)

	return app.db.Ping()
}

func (app *App) waitForDB(retries int) error {
	for i := 0; i < retries; i++ {
		if err := app.initDB(); err == nil {
			return nil
		}
		time.Sleep(1 * time.Second)
	}
	return fmt.Errorf("database unavailable after %d retries", retries)
}

func requestIDMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if reqID := r.Header.Get("x-request-id"); reqID != "" {
			w.Header().Set("x-request-id", reqID)
		}
		next.ServeHTTP(w, r)
	})
}

func main() {
	cfg := loadConfig()
	app := &App{cfg: cfg}

	// Ensure DB is ready before starting server
	if err := app.waitForDB(30); err != nil {
		fmt.Fprintf(os.Stderr, "Failed to connect to database: %v\n", err)
		os.Exit(1)
	}
	defer app.db.Close()

	mux := http.NewServeMux()
	mux.HandleFunc("/health", app.handleHealth)
	mux.HandleFunc("/", app.handleRoot)
	mux.HandleFunc("/info", app.handleInfo)
	mux.HandleFunc("/json/", app.handleJSON)
	mux.HandleFunc("/db/read/one", app.handleDBReadOne)
	mux.HandleFunc("/db/read/many", app.handleDBReadMany)
	mux.HandleFunc("/db/write/insert", app.handleDBWriteInsert)
	mux.HandleFunc("/files/", app.handleFiles)

	server := &http.Server{
		Addr:         ":" + cfg.Port,
		Handler:      requestIDMiddleware(mux),
		ReadTimeout:  10 * time.Second,
		WriteTimeout: 10 * time.Second,
		IdleTimeout:  60 * time.Second,
	}

	if err := server.ListenAndServe(); err != nil {
		fmt.Fprintf(os.Stderr, "Server failed: %v\n", err)
		os.Exit(1)
	}
}

func (app *App) handleHealth(w http.ResponseWriter, r *http.Request) {
	if err := app.db.Ping(); err != nil {
		w.WriteHeader(http.StatusServiceUnavailable)
		fmt.Fprint(w, "DB unavailable")
		return
	}
	w.WriteHeader(http.StatusOK)
	fmt.Fprint(w, "OK")
}

func (app *App) handleRoot(w http.ResponseWriter, r *http.Request) {
	if r.URL.Path != "/" {
		http.NotFound(w, r)
		return
	}
	w.WriteHeader(http.StatusOK)
	fmt.Fprint(w, "Hello, World!")
}

func (app *App) handleInfo(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
	fmt.Fprint(w, "1.21,hello_world,json,db_read_one,db_read_paging,db_write,static_files")
}

func (app *App) handleJSON(w http.ResponseWriter, r *http.Request) {
	// Path: /json/{from}/{to}
	tail := strings.TrimPrefix(r.URL.Path, "/json/")
	parts := strings.SplitN(tail, "/", 2)
	if len(parts) != 2 || parts[0] == "" || parts[1] == "" {
		http.Error(w, "invalid path", http.StatusBadRequest)
		return
	}
	from, to := parts[0], parts[1]

	body, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, "failed read body", http.StatusBadRequest)
		return
	}
	defer r.Body.Close()

	var data interface{}
	if err := json.Unmarshal(body, &data); err != nil {
		http.Error(w, "invalid json", http.StatusBadRequest)
		return
	}

	_ = replaceJSON(data, from, to)

	w.Header().Set("Content-Type", "application/json")
	if err := json.NewEncoder(w).Encode(data); err != nil {
		http.Error(w, "encode error", http.StatusInternalServerError)
	}
}

func replaceJSON(v interface{}, from, to string) int {
	switch x := v.(type) {
	case map[string]interface{}:
		count := 0
		for k, val := range x {
			if k == "servlet-name" {
				if s, ok := val.(string); ok && s == from {
					x[k] = to
					count++
				}
			} else {
				count += replaceJSON(val, from, to)
			}
		}
		return count
	case []interface{}:
		count := 0
		for _, e := range x {
			count += replaceJSON(e, from, to)
		}
		return count
	default:
		return 0
	}
}

type DBItem struct {
	ID        int    `json:"id"`
	Name      string `json:"name"`
	CreatedAt string `json:"created_at"`
	UpdatedAt string `json:"updated_at"`
}

func (app *App) handleDBReadOne(w http.ResponseWriter, r *http.Request) {
	idStr := r.URL.Query().Get("id")
	id, err := strconv.Atoi(idStr)
	if err != nil || id <= 0 {
		http.Error(w, "invalid id", http.StatusBadRequest)
		return
	}

	var name string
	var createdAt, updatedAt time.Time

	err = app.db.QueryRow("SELECT name, created_at, updated_at FROM hello_world WHERE id = $1", id).
		Scan(&name, &createdAt, &updatedAt)

	if err != nil {
		if err == sql.ErrNoRows {
			http.Error(w, "not found", http.StatusNotFound)
			return
		}
		http.Error(w, "db error", http.StatusInternalServerError)
		return
	}

	resp := DBItem{
		ID:        id,
		Name:      name,
		CreatedAt: createdAt.Format(time.RFC3339),
		UpdatedAt: updatedAt.Format(time.RFC3339),
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(resp)
}

func (app *App) handleDBReadMany(w http.ResponseWriter, r *http.Request) {
	offsetStr := r.URL.Query().Get("offset")
	offset, err := strconv.Atoi(offsetStr)
	if err != nil || offset < 0 {
		http.Error(w, "invalid offset", http.StatusBadRequest)
		return
	}

	limit := 50
	if l, err := strconv.Atoi(r.URL.Query().Get("limit")); err == nil && l > 0 {
		limit = l
	}

	rows, err := app.db.Query("SELECT id, name, created_at, updated_at FROM hello_world ORDER BY id ASC LIMIT $1 OFFSET $2", limit, offset)
	if err != nil {
		http.Error(w, "db error", http.StatusInternalServerError)
		return
	}
	defer rows.Close()

	items := make([]DBItem, 0, limit)
	for rows.Next() {
		var item DBItem
		var createdAt, updatedAt time.Time
		if err := rows.Scan(&item.ID, &item.Name, &createdAt, &updatedAt); err != nil {
			http.Error(w, "db error", http.StatusInternalServerError)
			return
		}
		item.CreatedAt = createdAt.Format(time.RFC3339)
		item.UpdatedAt = updatedAt.Format(time.RFC3339)
		items = append(items, item)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(items)
}

func (app *App) handleDBWriteInsert(w http.ResponseWriter, r *http.Request) {
	var name string
	if strings.Contains(r.Header.Get("Content-Type"), "application/json") {
		var payload struct {
			Name string `json:"name"`
		}
		if err := json.NewDecoder(r.Body).Decode(&payload); err == nil {
			name = payload.Name
		}
	}
	if name == "" {
		name = r.URL.Query().Get("name")
	}

	if name == "" {
		http.Error(w, "missing name", http.StatusBadRequest)
		return
	}

	var id int
	var createdAt, updatedAt time.Time
	err := app.db.QueryRow("INSERT INTO hello_world (name, created_at, updated_at) VALUES ($1, NOW(), NOW()) RETURNING id, created_at, updated_at", name).
		Scan(&id, &createdAt, &updatedAt)

	if err != nil {
		http.Error(w, "db error", http.StatusInternalServerError)
		return
	}

	resp := DBItem{
		ID:        id,
		Name:      name,
		CreatedAt: createdAt.Format(time.RFC3339),
		UpdatedAt: updatedAt.Format(time.RFC3339),
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(resp)
}

func (app *App) handleFiles(w http.ResponseWriter, r *http.Request) {
	filename := strings.TrimPrefix(r.URL.Path, "/files/")
	if filename == "" {
		http.Error(w, "missing filename", http.StatusBadRequest)
		return
	}

	// Security check: prevent directory traversal
	if strings.Contains(filename, "..") || strings.Contains(filename, "/") || strings.Contains(filename, "\\") {
		http.Error(w, "invalid filename", http.StatusBadRequest)
		return
	}

	// Whitelist allowed files for benchmark safety
	allowed := map[string]bool{
		"15kb.bin": true,
		"1mb.bin":  true,
		"10mb.bin": true,
	}
	if !allowed[filename] {
		http.Error(w, "not found", http.StatusNotFound)
		return
	}

	path := filepath.Join(app.cfg.DataDir, filename)
	w.Header().Set("Content-Type", "application/octet-stream")
	http.ServeFile(w, r, path)
}
