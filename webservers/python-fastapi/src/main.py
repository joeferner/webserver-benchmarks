from fastapi import FastAPI
from fastapi.responses import PlainTextResponse, FileResponse

app = FastAPI()


@app.get("/benchmark/health")
def get_health():
    return {}


@app.get("/benchmark/plain-text")
def get_plain_text():
    return PlainTextResponse("Hello, World!")


@app.get("/benchmark/download-binary")
def get_download_binary():
    return FileResponse("/assets/download-binary.png", media_type="image/png")
