from fastapi import FastAPI
from fastapi.responses import PlainTextResponse

app = FastAPI()


@app.get("/benchmark/health")
def get_health():
    return {}


@app.get("/benchmark/plain-text")
def get_plain_text():
    return PlainTextResponse("Hello, World!")
