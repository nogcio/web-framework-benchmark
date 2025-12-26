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

@app.middleware("http")
async def x_request_id_middleware(request: Request, call_next):
    response = await call_next(request)
    request_id = request.headers.get("x-request-id")
    if request_id:
        response.headers["x-request-id"] = request_id
    return response

@app.get("/health")
async def health():
    return PlainTextResponse("OK")

@app.get("/")
async def hello_world():
    return PlainTextResponse("Hello, World!")

class Servlet(BaseModel):
    servlet_name: str = Field(..., alias="servlet-name")
    servlet_class: str = Field(..., alias="servlet-class")
    init_param: Optional[Dict[str, Any]] = Field(None, alias="init-param")

class Taglib(BaseModel):
    taglib_uri: str = Field(..., alias="taglib-uri")
    taglib_location: str = Field(..., alias="taglib-location")

class WebApp(BaseModel):
    servlet: List[Servlet]
    servlet_mapping: Dict[str, str] = Field(..., alias="servlet-mapping")
    taglib: Taglib

class WebAppPayload(BaseModel):
    web_app: WebApp = Field(..., alias="web-app")

@app.post("/json/{from_val}/{to_val}", response_model=WebAppPayload, response_model_by_alias=True, response_model_exclude_none=True)
async def json_serialization(from_val: str, to_val: str, payload: WebAppPayload):
    for servlet in payload.web_app.servlet:
        if servlet.servlet_name == from_val:
            servlet.servlet_name = to_val
    return payload
