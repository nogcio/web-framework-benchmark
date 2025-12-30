package main

import (
	"os"

	"github.com/goccy/go-json"
	"github.com/gofiber/fiber/v2"
)

func main() {
	app := fiber.New(fiber.Config{
		DisableStartupMessage: true,
		JSONEncoder:           json.Marshal,
		JSONDecoder:           json.Unmarshal,
	})

	app.Get("/health", func(c *fiber.Ctx) error {
		return c.SendString("OK")
	})

	app.Get("/", func(c *fiber.Ctx) error {
		return c.SendString("Hello, World!")
	})

	app.Get("/plaintext", func(c *fiber.Ctx) error {
		return c.SendString("Hello, World!")
	})

	app.Post("/json/aggregate", func(c *fiber.Ctx) error {
		var orders []Order
		if err := c.BodyParser(&orders); err != nil {
			return c.SendStatus(fiber.StatusBadRequest)
		}

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

		return c.JSON(fiber.Map{
			"processedOrders": processedOrders,
			"results":         results,
			"categoryStats":   categoryStats,
		})
	})

	dataDir := os.Getenv("DATA_DIR")
	if dataDir == "" {
		dataDir = "benchmarks_data"
	}

	app.Static("/files", dataDir, fiber.Static{
		ByteRange: true,
	})

	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}

	app.Listen("0.0.0.0:" + port)
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
