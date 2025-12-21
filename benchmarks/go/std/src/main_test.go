package main

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"testing"
)

func TestHandleRoot(t *testing.T) {
	app := &App{}
	req, err := http.NewRequest("GET", "/", nil)
	if err != nil {
		t.Fatal(err)
	}

	rr := httptest.NewRecorder()
	handler := http.HandlerFunc(app.handleRoot)

	handler.ServeHTTP(rr, req)

	if status := rr.Code; status != http.StatusOK {
		t.Errorf("handler returned wrong status code: got %v want %v",
			status, http.StatusOK)
	}

	expected := "Hello, World!"
	if rr.Body.String() != expected {
		t.Errorf("handler returned unexpected body: got %v want %v",
			rr.Body.String(), expected)
	}
}

func TestHandleInfo(t *testing.T) {
	app := &App{}
	req, err := http.NewRequest("GET", "/info", nil)
	if err != nil {
		t.Fatal(err)
	}

	rr := httptest.NewRecorder()
	handler := http.HandlerFunc(app.handleInfo)

	handler.ServeHTTP(rr, req)

	if status := rr.Code; status != http.StatusOK {
		t.Errorf("handler returned wrong status code: got %v want %v",
			status, http.StatusOK)
	}

	expected := "1.21,hello_world,json,db_read_one,db_read_paging,db_write,static_files"
	if rr.Body.String() != expected {
		t.Errorf("handler returned unexpected body: got %v want %v",
			rr.Body.String(), expected)
	}
}

func TestHandleJSON(t *testing.T) {
	app := &App{}
	
	// Input JSON
	input := map[string]interface{}{
		"servlet-name": "oldValue",
		"other": "value",
	}
	body, _ := json.Marshal(input)

	// Request to replace "oldValue" with "newValue"
	req, err := http.NewRequest("POST", "/json/oldValue/newValue", bytes.NewBuffer(body))
	if err != nil {
		t.Fatal(err)
	}

	rr := httptest.NewRecorder()
	handler := http.HandlerFunc(app.handleJSON)

	handler.ServeHTTP(rr, req)

	if status := rr.Code; status != http.StatusOK {
		t.Errorf("handler returned wrong status code: got %v want %v",
			status, http.StatusOK)
	}

	var response map[string]interface{}
	if err := json.Unmarshal(rr.Body.Bytes(), &response); err != nil {
		t.Fatal(err)
	}

	if response["servlet-name"] != "newValue" {
		t.Errorf("handler did not replace value: got %v want %v",
			response["servlet-name"], "newValue")
	}
}

func TestHandleFiles(t *testing.T) {
	// Create a temporary directory for test files
	tmpDir, err := os.MkdirTemp("", "benchmark_test")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(tmpDir)

	// Create a dummy file that is allowed
	filename := "15kb.bin"
	content := []byte("test content")
	if err := os.WriteFile(filepath.Join(tmpDir, filename), content, 0644); err != nil {
		t.Fatal(err)
	}

	app := &App{
		cfg: Config{
			DataDir: tmpDir,
		},
	}

	req, err := http.NewRequest("GET", "/files/"+filename, nil)
	if err != nil {
		t.Fatal(err)
	}

	rr := httptest.NewRecorder()
	handler := http.HandlerFunc(app.handleFiles)

	handler.ServeHTTP(rr, req)

	if status := rr.Code; status != http.StatusOK {
		t.Errorf("handler returned wrong status code: got %v want %v",
			status, http.StatusOK)
	}

	if rr.Body.String() != string(content) {
		t.Errorf("handler returned unexpected body: got %v want %v",
			rr.Body.String(), string(content))
	}

	// Test disallowed file
	req, err = http.NewRequest("GET", "/files/forbidden.txt", nil)
	if err != nil {
		t.Fatal(err)
	}

	rr = httptest.NewRecorder()
	handler.ServeHTTP(rr, req)

	if status := rr.Code; status != http.StatusNotFound {
		t.Errorf("handler returned wrong status code for forbidden file: got %v want %v",
			status, http.StatusNotFound)
	}
}
