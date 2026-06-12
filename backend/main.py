from fastapi import FastAPI

app = FastAPI(title="Expert Base API")


@app.get("/hello")
def hello():
    return {"message": "Hello, world!"}
