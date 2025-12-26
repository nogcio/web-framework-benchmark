package main

import (
	"io"
	"net/http"
	"os"

	"github.com/gin-gonic/gin"
	_ "github.com/goccy/go-json"
)

func main() {
	gin.SetMode(gin.ReleaseMode)
	gin.DefaultWriter = io.Discard
	gin.DefaultErrorWriter = io.Discard
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

	r.GET("/", func(c *gin.Context) {
		c.String(http.StatusOK, "Hello, World!")
	})

	r.POST("/json/:from/:to", func(c *gin.Context) {
		from := c.Param("from")
		to := c.Param("to")

		var body Root
		if err := c.ShouldBindJSON(&body); err != nil {
			c.Status(http.StatusBadRequest)
			return
		}

		for i := range body.WebApp.Servlet {
			if body.WebApp.Servlet[i].ServletName == from {
				body.WebApp.Servlet[i].ServletName = to
			}
		}

		c.JSON(http.StatusOK, body)
	})

	dataDir := os.Getenv("DATA_DIR")
	if dataDir == "" {
		dataDir = "benchmarks_data"
	}

	r.Static("/files", dataDir)

	port := os.Getenv("PORT")
	if port == "" {
		port = "8000"
	}
	r.Run(":" + port)
}

type Root struct {
	WebApp WebApp `json:"web-app"`
}

type WebApp struct {
	Servlet        []Servlet         `json:"servlet"`
	ServletMapping map[string]string `json:"servlet-mapping"`
	Taglib         Taglib            `json:"taglib"`
}

type Servlet struct {
	ServletName  string                 `json:"servlet-name"`
	ServletClass string                 `json:"servlet-class"`
	InitParam    map[string]interface{} `json:"init-param,omitempty"`
}

type Taglib struct {
	TaglibURI      string `json:"taglib-uri"`
	TaglibLocation string `json:"taglib-location"`
}
