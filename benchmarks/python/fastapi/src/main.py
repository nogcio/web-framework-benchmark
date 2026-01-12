from typing import List, Dict, Optional, Any
from fastapi import FastAPI, Request
from fastapi.responses import PlainTextResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel, Field
import os

app = FastAPI(openapi_url=None, docs_url=None, redoc_url=None)

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

class OrderItem(BaseModel):
    quantity: int
    category: str

class Order(BaseModel):
    status: str
    amount: int
    country: str
    items: Optional[List[OrderItem]] = None

class AggregateResponse(BaseModel):
    processedOrders: int
    results: Dict[str, int]
    categoryStats: Dict[str, int]

@app.post("/json/aggregate", response_model=AggregateResponse)
async def json_aggregate(orders: List[Order]):
    processed_orders = 0
    results: Dict[str, int] = {}
    category_stats: Dict[str, int] = {}

    for order in orders:
        if order.status == "completed":
            processed_orders += 1
            
            # results: country -> amount
            results[order.country] = results.get(order.country, 0) + order.amount
            
            # category_stats: category -> quantity
            if order.items:
                for item in order.items:
                    category_stats[item.category] = category_stats.get(item.category, 0) + item.quantity

    return AggregateResponse(
        processedOrders=processed_orders,
        results=results,
        categoryStats=category_stats
    )
