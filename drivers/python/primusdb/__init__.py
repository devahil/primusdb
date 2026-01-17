"""
PrimusDB Python Driver

A high-performance Python client for PrimusDB, supporting all storage engines
and advanced features like AI/ML predictions and vector search.
"""

import asyncio
import json
from typing import Dict, List, Optional, Union, Any
from dataclasses import dataclass
from enum import Enum


class StorageType(Enum):
    """Storage engine types supported by PrimusDB"""
    COLUMNAR = "columnar"
    VECTOR = "vector"
    DOCUMENT = "document"
    RELATIONAL = "relational"


@dataclass
class ConnectionConfig:
    """Configuration for connecting to PrimusDB server"""
    host: str = "localhost"
    port: int = 8080
    timeout: float = 30.0
    max_connections: int = 10


class PrimusDBClient:
    """
    Main client for interacting with PrimusDB server.

    Supports all storage engines and advanced operations like AI predictions,
    vector search, and data clustering.
    """

    def __init__(self, config: Optional[ConnectionConfig] = None):
        """
        Initialize the PrimusDB client.

        Args:
            config: Connection configuration. If None, uses default settings.
        """
        self.config = config or ConnectionConfig()
        self._connected = False
        self._session = None

    async def connect(self) -> None:
        """
        Connect to the PrimusDB server.

        Raises:
            ConnectionError: If connection fails
        """
        try:
            import aiohttp
            self._session = aiohttp.ClientSession(
                timeout=aiohttp.ClientTimeout(total=self.config.timeout)
            )
            self._connected = True
        except ImportError:
            raise ImportError("aiohttp is required for PrimusDB client")

    async def close(self) -> None:
        """Close the connection to the server."""
        if self._session:
            await self._session.close()
            self._connected = False

    async def _request(self, method: str, endpoint: str, data: Optional[Dict] = None) -> Dict:
        """Make an HTTP request to the PrimusDB server."""
        if not self._connected:
            raise ConnectionError("Not connected to PrimusDB server")

        url = f"http://{self.config.host}:{self.config.port}/api/v1/{endpoint}"

        async with self._session.request(method, url, json=data) as response:
            result = await response.json()

            if not result.get("success", False):
                error_msg = result.get("error", "Unknown error")
                raise RuntimeError(f"PrimusDB error: {error_msg}")

            return result.get("data")

    async def create_table(self, storage_type: StorageType, table: str, schema: Dict) -> None:
        """
        Create a new table/collection.

        Args:
            storage_type: Type of storage engine to use
            table: Name of the table/collection
            schema: Schema definition as a dictionary
        """
        endpoint = f"table/{storage_type.value}/{table}"
        await self._request("POST", endpoint, {"schema": schema})

    async def insert(self, storage_type: StorageType, table: str, data: Dict) -> int:
        """
        Insert a record into the specified table.

        Args:
            storage_type: Type of storage engine
            table: Name of the table/collection
            data: Data to insert

        Returns:
            Number of records inserted (usually 1)
        """
        endpoint = f"crud/{storage_type.value}/{table}"
        result = await self._request("POST", endpoint, {"data": data})
        return result.get("count", 0)

    async def select(self, storage_type: StorageType, table: str,
                    conditions: Optional[Dict] = None,
                    limit: Optional[int] = None,
                    offset: Optional[int] = None) -> List[Dict]:
        """
        Select records from the specified table.

        Args:
            storage_type: Type of storage engine
            table: Name of the table/collection
            conditions: Query conditions
            limit: Maximum number of records to return
            offset: Number of records to skip

        Returns:
            List of matching records
        """
        params = {}
        if conditions:
            params["conditions"] = json.dumps(conditions)
        if limit:
            params["limit"] = str(limit)
        if offset:
            params["offset"] = str(offset)

        query_string = "&".join(f"{k}={v}" for k, v in params.items())
        endpoint = f"crud/{storage_type.value}/{table}"
        if query_string:
            endpoint += f"?{query_string}"

        return await self._request("GET", endpoint)

    async def update(self, storage_type: StorageType, table: str,
                    conditions: Optional[Dict], data: Dict) -> int:
        """
        Update records in the specified table.

        Args:
            storage_type: Type of storage engine
            table: Name of the table/collection
            conditions: Conditions to match records for update
            data: New data to set

        Returns:
            Number of records updated
        """
        endpoint = f"crud/{storage_type.value}/{table}"
        payload = {"data": data}
        if conditions:
            payload["conditions"] = conditions

        result = await self._request("PUT", endpoint, payload)
        return result.get("count", 0)

    async def delete(self, storage_type: StorageType, table: str,
                    conditions: Optional[Dict] = None) -> int:
        """
        Delete records from the specified table.

        Args:
            storage_type: Type of storage engine
            table: Name of the table/collection
            conditions: Conditions to match records for deletion

        Returns:
            Number of records deleted
        """
        params = {}
        if conditions:
            params["conditions"] = json.dumps(conditions)

        query_string = "&".join(f"{k}={v}" for k, v in params.items())
        endpoint = f"crud/{storage_type.value}/{table}"
        if query_string:
            endpoint += f"?{query_string}"

        result = await self._request("DELETE", endpoint)
        return result.get("count", 0)

    async def analyze(self, storage_type: StorageType, table: str,
                     conditions: Optional[Dict] = None) -> Dict:
        """
        Analyze data patterns in the specified table.

        Args:
            storage_type: Type of storage engine
            table: Name of the table/collection
            conditions: Analysis conditions

        Returns:
            Analysis results
        """
        endpoint = f"advanced/analyze/{storage_type.value}/{table}"
        payload = {}
        if conditions:
            payload["conditions"] = conditions

        return await self._request("POST", endpoint, payload)

    async def predict(self, storage_type: StorageType, table: str,
                     data: Dict, prediction_type: str = "linear_regression") -> Dict:
        """
        Make AI predictions using trained models.

        Args:
            storage_type: Type of storage engine
            table: Name of the table/collection
            data: Input data for prediction
            prediction_type: Type of prediction algorithm

        Returns:
            Prediction results
        """
        endpoint = f"advanced/predict/{storage_type.value}/{table}"
        payload = {
            "data": data,
            "prediction_type": prediction_type
        }

        return await self._request("POST", endpoint, payload)

    async def vector_search(self, table: str, query_vector: List[float],
                           limit: int = 10) -> List[Dict]:
        """
        Perform vector similarity search.

        Args:
            table: Name of the vector table/collection
            query_vector: Query vector as list of floats
            limit: Maximum number of results

        Returns:
            List of similar vectors with metadata
        """
        endpoint = f"advanced/vector-search/{table}"
        payload = {
            "query_vector": query_vector,
            "limit": limit
        }

        return await self._request("POST", endpoint, payload)

    async def cluster(self, storage_type: StorageType, table: str,
                     params: Optional[Dict] = None) -> Dict:
        """
        Perform data clustering analysis.

        Args:
            storage_type: Type of storage engine
            table: Name of the table/collection
            params: Clustering parameters

        Returns:
            Clustering results
        """
        endpoint = f"advanced/cluster/{storage_type.value}/{table}"
        payload = params or {"algorithm": "kmeans", "clusters": 5}

        return await self._request("POST", endpoint, payload)

    async def __aenter__(self):
        await self.connect()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.close()


class Collection:
    """
    High-level collection abstraction for easier data operations.
    """

    def __init__(self, client: PrimusDBClient, storage_type: StorageType, name: str):
        self.client = client
        self.storage_type = storage_type
        self.name = name

    async def insert_one(self, data: Dict) -> int:
        """Insert a single document/record."""
        return await self.client.insert(self.storage_type, self.name, data)

    async def find(self, conditions: Optional[Dict] = None,
                  limit: Optional[int] = None,
                  offset: Optional[int] = None) -> List[Dict]:
        """Find documents/records matching conditions."""
        return await self.client.select(self.storage_type, self.name, conditions, limit, offset)

    async def update_one(self, conditions: Optional[Dict], data: Dict) -> int:
        """Update a single document/record."""
        return await self.client.update(self.storage_type, self.name, conditions, data)

    async def delete_one(self, conditions: Optional[Dict]) -> int:
        """Delete documents/records matching conditions."""
        return await self.client.delete(self.storage_type, self.name, conditions)

    async def count(self, conditions: Optional[Dict] = None) -> int:
        """Count documents/records matching conditions."""
        results = await self.find(conditions, limit=1000000)
        return len(results)


# Convenience functions
async def connect(host: str = "localhost", port: int = 8080) -> PrimusDBClient:
    """
    Create and connect a new PrimusDB client.

    Args:
        host: Server hostname
        port: Server port

    Returns:
        Connected PrimusDB client
    """
    config = ConnectionConfig(host=host, port=port)
    client = PrimusDBClient(config)
    await client.connect()
    return client


__all__ = [
    "PrimusDBClient",
    "StorageType",
    "ConnectionConfig",
    "Collection",
    "connect",
]