import requests
import json

BASE_URL = "http://localhost:8080"


def main():
    print("=== PrimusDB Python Example ===")

    # Check health
    try:
        resp = requests.get(f"{BASE_URL}/health", timeout=5)
        resp.raise_for_status()
        print(f"Health: {resp.json()}")
    except requests.RequestException as e:
        print(f"Error connecting to PrimusDB: {e}")
        return

    # Get version
    try:
        resp = requests.get(f"{BASE_URL}/version", timeout=5)
        resp.raise_for_status()
        print(f"Version: {resp.json()}")
    except requests.RequestException as e:
        print(f"Error fetching version: {e}")

    # Create a record
    record = {
        "collection": "users",
        "data": {
            "name": "Alice",
            "email": "alice@example.com",
        },
    }
    try:
        resp = requests.post(f"{BASE_URL}/records", json=record, timeout=5)
        resp.raise_for_status()
        print(f"Created record: {resp.json()}")
    except requests.RequestException as e:
        print(f"Error creating record: {e}")

    # Query records
    try:
        resp = requests.get(f"{BASE_URL}/records/users", timeout=5)
        resp.raise_for_status()
        print(f"Records: {resp.json()}")
    except requests.RequestException as e:
        print(f"Error querying records: {e}")


if __name__ == "__main__":
    main()
