from fastapi import FastAPI

app = FastAPI()

@app.get("/benchmark/health")
def get_health():
    return {}
