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

	r.GET("/health", func(c *gin.Context) {
		c.String(http.StatusOK, "OK")
	})

	r.GET("/", func(c *gin.Context) {
		c.String(http.StatusOK, "Hello, World!")
	})

	r.GET("/plaintext", func(c *gin.Context) {
		c.String(http.StatusOK, "Hello, World!")
	})

	r.POST("/json/aggregate", func(c *gin.Context) {
		var orders []Order
		_ = c.ShouldBindJSON(&orders)

		processedOrders := 0
		results := make(map[string]int64)
		categoryStats := make(map[string]int32)

		for _, order := range orders {
			if order.Status == "completed" {
				processedOrders++
				results[order.Country] += order.Amount
				for _, item := range order.Items {
					categoryStats[item.Category] += item.Quantity
				}
			}
		}

		c.JSON(http.StatusOK, gin.H{
			"processedOrders": processedOrders,
			"results":         results,
			"categoryStats":   categoryStats,
		})
	})

	dataDir := os.Getenv("DATA_DIR")
	if dataDir == "" {
		dataDir = "benchmarks_data"
	}

	r.Static("/files", dataDir)

	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}
	r.Run(":" + port)
}

type Order struct {
	Status  string      `json:"status"`
	Amount  int64       `json:"amount"`
	Country string      `json:"country"`
	Items   []OrderItem `json:"items"`
}

type OrderItem struct {
	Quantity int32  `json:"quantity"`
	Category string `json:"category"`
}
