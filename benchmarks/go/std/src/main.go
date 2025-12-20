package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"strconv"
	"strings"
	"time"
	"io"

	_ "github.com/lib/pq"
)

var db *sql.DB

func initDB() error {
	host := os.Getenv("DB_HOST")
	if host == "" {
		host = "db"
	}
	user := os.Getenv("DB_USER")
	if user == "" {
		user = "benchmark"
	}
	pass := os.Getenv("DB_PASSWORD")
	if pass == "" {
		pass = "benchmark"
	}
	name := os.Getenv("DB_NAME")
	if name == "" {
		name = "benchmark"
	}
	port := os.Getenv("DB_PORT")
	if port == "" {
		port = "5432"
	}
	conn := fmt.Sprintf("postgres://%s:%s@%s:%s/%s?sslmode=disable", user, pass, host, port, name)
	var err error
	db, err = sql.Open("postgres", conn)
	if err != nil {
		return err
	}
	db.SetMaxOpenConns(25)
	db.SetConnMaxIdleTime(5 * time.Minute)
	return db.Ping()
}

func main() {
	// try to init DB but don't fail server start if DB isn't ready yet
	go func() {
		for i := 0; i < 10; i++ {
			if err := initDB(); err == nil {
				return
			}
			time.Sleep(500 * time.Millisecond)
		}
	}()

	mux := http.NewServeMux()

	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		// Return OK only when DB is ready
		dbReady := false
		if db == nil {
			if err := initDB(); err == nil {
				dbReady = true
			}
		} else {
			if err := db.Ping(); err == nil {
				dbReady = true
			}
		}
		if dbReady {
			w.WriteHeader(http.StatusOK)
			fmt.Fprint(w, "OK")
			return
		}
		w.WriteHeader(http.StatusServiceUnavailable)
		fmt.Fprint(w, "DB unavailable")
	})

	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		fmt.Fprint(w, "Hello, World!")
	})


	// Info endpoint: comma-separated supported tests, then version as last element.
	// Format expected by the benchmark harness: "version,comma-separated-tests"
	mux.HandleFunc("/info", func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		fmt.Fprint(w, "1.21,hello_world,json,db_read_one,db_read_paging,db_write,static_files")
	})


	// JSON endpoint: supports GET /json/{r1}/{r2} (original numeric echo)
	// and POST /json/{from}/{to} which replaces servlet-name occurrences
	mux.HandleFunc("/json/", func(w http.ResponseWriter, r *http.Request) {
		// path after /json/
		tail := r.URL.Path[len("/json/"):]
		parts := strings.SplitN(tail, "/", 2)
		// expect /json/{from}/{to}
		if len(parts) != 2 || parts[0] == "" || parts[1] == "" {
			w.WriteHeader(http.StatusBadRequest)
			fmt.Fprint(w, "invalid path")
			return
		}
		from := parts[0]
		to := parts[1]
		// echo request id header
		reqid := r.Header.Get("x-request-id")
		if reqid != "" {
			w.Header().Set("x-request-id", reqid)
		}
		defer r.Body.Close()
		body, err := io.ReadAll(r.Body)
		if err != nil {
			w.WriteHeader(http.StatusBadRequest)
			fmt.Fprint(w, "failed read body")
			return
		}
		var data interface{}
		if err := json.Unmarshal(body, &data); err != nil {
			w.WriteHeader(http.StatusBadRequest)
			fmt.Fprint(w, "invalid json")
			return
		}
		// recursive replace function
		var replace func(interface{}) int
		replace = func(v interface{}) int {
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
						count += replace(val)
					}
				}
				return count
			case []interface{}:
				count := 0
				for _, e := range x {
					count += replace(e)
				}
				return count
			default:
				return 0
			}
		}
		_ = replace(data)
		out, err := json.Marshal(data)
		if err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			fmt.Fprint(w, "encode error")
			return
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write(out)
		return
	})

	// DB read one: /db/read/one?id=N
	mux.HandleFunc("/db/read/one", func(w http.ResponseWriter, r *http.Request) {
		// echo request id header
		reqid := r.Header.Get("x-request-id")
		if reqid != "" {
			w.Header().Set("x-request-id", reqid)
		}
		idStr := r.URL.Query().Get("id")
		if idStr == "" {
			w.WriteHeader(http.StatusBadRequest)
			fmt.Fprint(w, "missing id")
			return
		}
		id, err := strconv.Atoi(idStr)
		if err != nil || id <= 0 {
			w.WriteHeader(http.StatusBadRequest)
			fmt.Fprint(w, "invalid id")
			return
		}
		// ensure DB is initialized
		if db == nil {
			if err := initDB(); err != nil {
				w.WriteHeader(http.StatusInternalServerError)
				fmt.Fprint(w, "db not available")
				return
			}
		}
		var name string
		var createdAt time.Time
		var updatedAt time.Time
		row := db.QueryRow("SELECT name, created_at, updated_at FROM hello_world WHERE id = $1", id)
		if err := row.Scan(&name, &createdAt, &updatedAt); err != nil {
			if err == sql.ErrNoRows {
				w.WriteHeader(http.StatusNotFound)
				fmt.Fprint(w, "not found")
				return
			}
			w.WriteHeader(http.StatusInternalServerError)
			fmt.Fprint(w, "db error")
			return
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		resp := struct {
			ID        int    `json:"id"`
			Name      string `json:"name"`
			CreatedAt string `json:"created_at"`
			UpdatedAt string `json:"updated_at"`
		}{
			ID:        id,
			Name:      name,
			CreatedAt: createdAt.Format(time.RFC3339),
			UpdatedAt: updatedAt.Format(time.RFC3339),
		}
		_ = json.NewEncoder(w).Encode(resp)
	})

	// DB read many: /db/read/many?offset=N&limit=M
	mux.HandleFunc("/db/read/many", func(w http.ResponseWriter, r *http.Request) {
		reqid := r.Header.Get("x-request-id")
		if reqid != "" {
			w.Header().Set("x-request-id", reqid)
		}
		offsetStr := r.URL.Query().Get("offset")
		limitStr := r.URL.Query().Get("limit")
		if offsetStr == "" {
			w.WriteHeader(http.StatusBadRequest)
			fmt.Fprint(w, "missing offset")
			return
		}
		offset, err := strconv.Atoi(offsetStr)
		if err != nil || offset < 0 {
			w.WriteHeader(http.StatusBadRequest)
			fmt.Fprint(w, "invalid offset")
			return
		}
		limit := 50
		if limitStr != "" {
			if l, err := strconv.Atoi(limitStr); err == nil && l > 0 {
				limit = l
			}
		}
		if db == nil {
			if err := initDB(); err != nil {
				w.WriteHeader(http.StatusInternalServerError)
				fmt.Fprint(w, "db not available")
				return
			}
		}
		rows, err := db.Query("SELECT id, name, created_at, updated_at FROM hello_world ORDER BY id ASC LIMIT $1 OFFSET $2", limit, offset)
		if err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			fmt.Fprint(w, "db error")
			return
		}
		defer rows.Close()
		type Item struct {
			ID        int    `json:"id"`
			Name      string `json:"name"`
			CreatedAt string `json:"created_at"`
			UpdatedAt string `json:"updated_at"`
		}
		items := make([]Item, 0, limit)
		for rows.Next() {
			var id int
			var name string
			var createdAt time.Time
			var updatedAt time.Time
			if err := rows.Scan(&id, &name, &createdAt, &updatedAt); err != nil {
				w.WriteHeader(http.StatusInternalServerError)
				fmt.Fprint(w, "db error")
				return
			}
			items = append(items, Item{
				ID:        id,
				Name:      name,
				CreatedAt: createdAt.Format(time.RFC3339),
				UpdatedAt: updatedAt.Format(time.RFC3339),
			})
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		_ = json.NewEncoder(w).Encode(items)
	})

	// DB write/insert: supports either POST JSON body {"name":"..."}
	mux.HandleFunc("/db/write/insert", func(w http.ResponseWriter, r *http.Request) {
		reqid := r.Header.Get("x-request-id")
		if reqid != "" {
			w.Header().Set("x-request-id", reqid)
		}

		var name string

		// attempt to decode JSON body if Content-Type indicates JSON
		if strings.Contains(r.Header.Get("Content-Type"), "application/json") {
			var payload struct {
				Name string `json:"name"`
			}
			if err := json.NewDecoder(r.Body).Decode(&payload); err == nil {
				name = payload.Name
			}
			// ensure body is closed (http server does that), but keep defensive
			_ = r.Body.Close()
		}
		// fallback to query param if body didn't include name
		if name == "" {
			name = r.URL.Query().Get("name")
		}

		if name == "" {
			w.WriteHeader(http.StatusBadRequest)
			fmt.Fprint(w, "missing name")
			return
		}

		if db == nil {
			if err := initDB(); err != nil {
				w.WriteHeader(http.StatusInternalServerError)
				fmt.Fprint(w, "db not available")
				return
			}
		}

		var id int
		var createdAt time.Time
		var updatedAt time.Time
		err := db.QueryRow("INSERT INTO hello_world (name, created_at, updated_at) VALUES ($1, NOW(), NOW()) RETURNING id, created_at, updated_at", name).Scan(&id, &createdAt, &updatedAt)
		if err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			fmt.Fprint(w, "db error")
			return
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		resp := struct {
			ID        int    `json:"id"`
			Name      string `json:"name"`
			CreatedAt string `json:"created_at"`
			UpdatedAt string `json:"updated_at"`
		}{
			ID:        id,
			Name:      name,
			CreatedAt: createdAt.Format(time.RFC3339),
			UpdatedAt: updatedAt.Format(time.RFC3339),
		}
		_ = json.NewEncoder(w).Encode(resp)
	})

	// Serve files from disk: /files/{filename}
	mux.HandleFunc("/files/", func(w http.ResponseWriter, r *http.Request) {
		// path after /files/
		tail := r.URL.Path[len("/files/"):]
		if tail == "" {
			w.WriteHeader(http.StatusBadRequest)
			fmt.Fprint(w, "missing filename")
			return
		}
		// only allow specific filenames for safety
		allowed := map[string]bool{
			"15kb.bin": true,
			"1mb.bin":  true,
			"10mb.bin": true,
		}
		if !allowed[tail] {
			w.WriteHeader(http.StatusNotFound)
			fmt.Fprint(w, "not found")
			return
		}
		// file path under benchmarks_data
		fpath := fmt.Sprintf("benchmarks_data/%s", tail)
		f, err := os.Open(fpath)
		if err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			fmt.Fprint(w, "file open error")
			return
		}
		defer f.Close()
		// set headers
		fi, err := f.Stat()
		if err == nil {
			w.Header().Set("Content-Length", strconv.FormatInt(fi.Size(), 10))
		}
		w.Header().Set("Content-Type", "application/octet-stream")
		w.WriteHeader(http.StatusOK)
		// stream file
		_, _ = io.Copy(w, f)
	})
	http.ListenAndServe(":8000", mux)
}