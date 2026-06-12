import sys
from pathlib import Path

from fastapi.testclient import TestClient

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from main import app


def test_hello_returns_message():
    client = TestClient(app)

    response = client.get("/hello")

    assert response.status_code == 200
    assert response.json() == {"message": "Hello, world!"}
