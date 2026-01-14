from typing import List, Dict, Optional, Any
from fastapi import FastAPI, Request
from fastapi.responses import PlainTextResponse, ORJSONResponse
from fastapi.staticfiles import StaticFiles
import os

app = FastAPI(default_response_class=ORJSONResponse, openapi_url=None, docs_url=None, redoc_url=None)

# Static files
data_dir = os.getenv("DATA_DIR", "benchmarks_data")
if os.path.exists(data_dir):
    app.mount("/files", StaticFiles(directory=data_dir), name="files")

@app.get("/health")
async def health():
    return PlainTextResponse("OK")

@app.get("/")
async def hello_world():
    return PlainTextResponse("Hello, World!")

@app.get("/plaintext")
async def plaintext():
    return PlainTextResponse("Hello, World!")

@app.post("/json/aggregate")
async def json_aggregate(request: Request):
    # Parse JSON manually to avoid Pydantic overhead
    # This matches the behavior of Django, Rails, Express, etc.
    try:
        orders = await request.json()
    except Exception:
        return PlainTextResponse("Invalid JSON", status_code=400)

    processed_orders = 0
    results: Dict[str, int] = {}
    category_stats: Dict[str, int] = {}

    for order in orders:
        if order.get("status") == "completed":
            processed_orders += 1
            
            # results: country -> amount
            country = order.get("country")
            amount = order.get("amount", 0)
            if country:
                 results[country] = results.get(country, 0) + amount
            
            # category_stats: category -> quantity
            items = order.get("items")
            if items:
                for item in items:
                    category = item.get("category")
                    quantity = item.get("quantity", 0)
                    if category:
                        category_stats[category] = category_stats.get(category, 0) + quantity

    return {
        "processedOrders": processed_orders,
        "results": results,
        "categoryStats": category_stats
    }
