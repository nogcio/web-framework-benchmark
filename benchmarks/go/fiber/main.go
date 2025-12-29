package main

import (
	"os"

	"github.com/gofiber/fiber/v2"
)

func main() {
	app := fiber.New(fiber.Config{
		DisableStartupMessage: true,
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
