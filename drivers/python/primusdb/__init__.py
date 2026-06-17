"""
PrimusDB Python Driver

A high-performance Python client for PrimusDB, supporting all storage engines
and advanced features like AI/ML predictions and vector search.
"""

import asyncio
import json
import urllib.parse
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
        endpoint = f"advanced/cluster/{storage_type.value}/{table}"
        payload = params or {"algorithm": "kmeans", "clusters": 5}
        return await self._request("POST", endpoint, payload)

    # ==================== Cluster Gateway Methods ====================

    async def cluster_status(self) -> Dict:
        endpoint = "cluster/status"
        return await self._request("GET", endpoint)

    async def cluster_nodes(self) -> List[Dict]:
        endpoint = "cluster/nodes"
        return await self._request("GET", endpoint)

    async def route_request(self, shard_key: Optional[str] = None,
                            preferred_nodes: Optional[List[str]] = None) -> Dict:
        payload = {}
        if shard_key is not None:
            payload["shard_key"] = shard_key
        if preferred_nodes is not None:
            payload["preferred_nodes"] = preferred_nodes
        endpoint = "cluster/route"
        return await self._request("POST", endpoint, payload)

    async def cluster_metrics(self) -> Dict:
        endpoint = "cluster/metrics"
        return await self._request("GET", endpoint)

    # ==================== Federation Methods ====================

    async def federation_status(self) -> Dict:
        endpoint = "federation/status"
        return await self._request("GET", endpoint)

    async def federation_clusters(self) -> List[Dict]:
        endpoint = "federation/clusters"
        return await self._request("GET", endpoint)

    async def federation_domains(self) -> List[Dict]:
        endpoint = "federation/domains"
        return await self._request("GET", endpoint)

    async def federation_metrics(self) -> Dict:
        endpoint = "federation/metrics"
        return await self._request("GET", endpoint)

    async def create_data_domain(self, name: str, description: Optional[str] = None,
                                  replication_mode: Optional[str] = None,
                                  storage_types: Optional[List[str]] = None,
                                  collections: Optional[List[str]] = None,
                                  tables: Optional[List[str]] = None,
                                  member_clusters: Optional[List[str]] = None) -> Dict:
        endpoint = "federation/domains"
        payload = {
            "name": name,
            "description": description,
            "replication_mode": replication_mode,
            "storage_types": storage_types or [],
            "collections": collections or [],
            "tables": tables or [],
            "member_clusters": member_clusters or [],
        }
        return await self._request("POST", endpoint, payload)

    async def join_domain(self, name: str, collections: Optional[List[str]] = None,
                           storage_types: Optional[List[str]] = None,
                           replication_mode: Optional[str] = None) -> Dict:
        endpoint = f"federation/domains/{name}/join"
        payload = {
            "collections": collections,
            "storage_types": storage_types,
            "replication_mode": replication_mode,
        }
        return await self._request("POST", endpoint, payload)

    async def leave_domain(self, name: str) -> Dict:
        endpoint = f"federation/domains/{name}/leave"
        return await self._request("POST", endpoint, {})

    async def balance_domain(self, name: str) -> Dict:
        endpoint = f"federation/domains/{name}/balance"
        return await self._request("POST", endpoint, {})

    # ==================== Resource Governor Methods ====================

    async def governor_start_execution(
        self, namespace, workload_type, user=None, role=None
    ):
        """Start a governor-tracked execution."""
        payload = {
            "namespace": namespace,
            "workload_type": workload_type,
        }
        if user is not None:
            payload["user"] = user
        if role is not None:
            payload["role"] = role
        async with self._session.post(
            f"http://{self.config.host}:{self.config.port}/governor/executions/start",
            json=payload,
        ) as resp:
            data = await resp.json()
            if not data.get("success"):
                raise RuntimeError(data.get("error", "Governor start failed"))
            return data["data"]

    async def governor_finish_execution(self, execution_id):
        """Finish a governor-tracked execution."""
        async with self._session.post(
            f"http://{self.config.host}:{self.config.port}/governor/executions/{execution_id}/finish",
            json={},
        ) as resp:
            data = await resp.json()
            if not data.get("success"):
                raise RuntimeError(data.get("error", "Governor finish failed"))

    async def governor_check_limit(self, execution_id, check_type, value):
        """Check a resource limit for an execution."""
        payload = {"check_type": check_type, "value": value}
        async with self._session.post(
            f"http://{self.config.host}:{self.config.port}/governor/executions/{execution_id}/check",
            json=payload,
        ) as resp:
            data = await resp.json()
            if not data.get("success"):
                raise RuntimeError(data.get("error", "Governor check failed"))
            return data["data"]

    async def governor_status(self):
        """Get governor status."""
        async with self._session.get(
            f"http://{self.config.host}:{self.config.port}/governor/status",
        ) as resp:
            data = await resp.json()
            if not data.get("success"):
                raise RuntimeError(data.get("error", "Governor status failed"))
            return data["data"]

    async def governor_metrics(self):
        """Get governor metrics snapshot."""
        async with self._session.get(
            f"http://{self.config.host}:{self.config.port}/governor/metrics",
        ) as resp:
            data = await resp.json()
            if not data.get("success"):
                raise RuntimeError(data.get("error", "Governor metrics failed"))
            return data["data"]

    async def governor_list_executions(self):
        """List all active executions."""
        async with self._session.get(
            f"http://{self.config.host}:{self.config.port}/governor/executions",
        ) as resp:
            data = await resp.json()
            if not data.get("success"):
                raise RuntimeError(data.get("error", "Governor list executions failed"))
            return data["data"]

    async def governor_list_violations(self):
        """List all violations."""
        async with self._session.get(
            f"http://{self.config.host}:{self.config.port}/governor/violations",
        ) as resp:
            data = await resp.json()
            if not data.get("success"):
                raise RuntimeError(data.get("error", "Governor list violations failed"))
            return data["data"]

    async def governor_policies(self):
        """List all policies."""
        async with self._session.get(
            f"http://{self.config.host}:{self.config.port}/governor/policies",
        ) as resp:
            data = await resp.json()
            if not data.get("success"):
                raise RuntimeError(data.get("error", "Governor policies failed"))
            return data["data"]

    async def governor_update_policy(self, name, limits, action, scope):
        """Create or update a resource policy."""
        payload = {
            "name": name,
            "limits": limits,
            "action": action,
            "scope": scope,
        }
        async with self._session.post(
            f"http://{self.config.host}:{self.config.port}/governor/policies/update",
            json=payload,
        ) as resp:
            data = await resp.json()
            if not data.get("success"):
                raise RuntimeError(data.get("error", "Governor update policy failed"))

    # ==================== DDL / ER Model Operations ====================

    async def add_column(self, storage_type: StorageType, table: str, field: Dict) -> Dict:
        endpoint = f"ddl/{storage_type.value}/{table}/column/add"
        return await self._request("POST", endpoint, field)

    async def drop_column(self, storage_type: StorageType, table: str, column_name: str) -> Dict:
        endpoint = f"ddl/{storage_type.value}/{table}/column/{column_name}"
        return await self._request("DELETE", endpoint)

    async def modify_column(self, storage_type: StorageType, table: str, field: Dict) -> Dict:
        endpoint = f"ddl/{storage_type.value}/{table}/column"
        return await self._request("PUT", endpoint, field)

    async def add_constraint(self, storage_type: StorageType, table: str, constraint: Dict) -> Dict:
        endpoint = f"ddl/{storage_type.value}/{table}/constraint"
        return await self._request("POST", endpoint, constraint)

    async def drop_constraint(self, storage_type: StorageType, table: str, constraint_name: str) -> Dict:
        endpoint = f"ddl/{storage_type.value}/{table}/constraint/{constraint_name}"
        return await self._request("DELETE", endpoint)

    async def rename_table(self, storage_type: StorageType, table: str, new_name: str) -> Dict:
        endpoint = f"ddl/{storage_type.value}/{table}/rename"
        return await self._request("POST", endpoint, {"new_name": new_name})

    async def create_sequence(self, storage_type: StorageType, sequence: Dict) -> Dict:
        endpoint = f"sequence/{storage_type.value}"
        return await self._request("POST", endpoint, sequence)

    async def drop_sequence(self, storage_type: StorageType, name: str) -> Dict:
        endpoint = f"sequence/{storage_type.value}/{name}"
        return await self._request("DELETE", endpoint)

    async def nextval(self, storage_type: StorageType, name: str) -> Dict:
        endpoint = f"sequence/{storage_type.value}/{name}/nextval"
        return await self._request("POST", endpoint)

    async def currval(self, storage_type: StorageType, name: str) -> Dict:
        endpoint = f"sequence/{storage_type.value}/{name}/currval"
        return await self._request("GET", endpoint)

    async def setval(self, storage_type: StorageType, name: str, value: int) -> Dict:
        endpoint = f"sequence/{storage_type.value}/{name}/setval"
        return await self._request("POST", endpoint, {"value": value})

    async def create_view(self, storage_type: StorageType, view: Dict) -> Dict:
        endpoint = f"view/{storage_type.value}"
        return await self._request("POST", endpoint, view)

    async def drop_view(self, storage_type: StorageType, name: str) -> Dict:
        endpoint = f"view/{storage_type.value}/{name}"
        return await self._request("DELETE", endpoint)

    async def refresh_view(self, storage_type: StorageType, name: str) -> Dict:
        endpoint = f"view/{storage_type.value}/{name}/refresh"
        return await self._request("POST", endpoint)

    async def create_trigger(self, storage_type: StorageType, table: str, trigger: Dict) -> Dict:
        endpoint = f"trigger/{storage_type.value}/{table}"
        return await self._request("POST", endpoint, trigger)

    async def drop_trigger(self, storage_type: StorageType, table: str, trigger_name: str) -> Dict:
        endpoint = f"trigger/{storage_type.value}/{table}/{trigger_name}"
        return await self._request("DELETE", endpoint)

    async def info_schema_tables(self, storage_type: StorageType) -> Dict:
        endpoint = f"info-schema/{storage_type.value}/tables"
        return await self._request("GET", endpoint)

    async def info_schema_columns(self, storage_type: StorageType, table: str) -> Dict:
        endpoint = f"info-schema/{storage_type.value}/{table}/columns"
        return await self._request("GET", endpoint)

    async def info_schema_constraints(self, storage_type: StorageType, table: str) -> Dict:
        endpoint = f"info-schema/{storage_type.value}/{table}/constraints"
        return await self._request("GET", endpoint)

    # ==================== UQL / SQL Execution ====================

    async def execute_sql(self, sql: str, params: Optional[Dict] = None) -> Dict:
        endpoint = "uql"
        payload = {"query": sql, "language": "sql"}
        if params:
            payload["params"] = params
        return await self._request("POST", endpoint, payload)

    # ==================== ER Model Features (v1.2.2+) ====================

    async def truncate_table(self, storage_type: StorageType, table: str, cascade: bool = False) -> Dict:
        endpoint = f"crud/{storage_type.value}/{table}/truncate"
        return await self._request("POST", endpoint, {"cascade": cascade})

    async def insert_returning(self, storage_type: StorageType, table: str,
                               data: Dict, returning: List[str]) -> List[Dict]:
        cols = ", ".join(data.keys())
        vals = ", ".join(f"'{v}'" if isinstance(v, str) else str(v) for v in data.values())
        ret = ", ".join(returning)
        sql = f"INSERT INTO {table} ({cols}) VALUES ({vals}) RETURNING {ret}"
        result = await self.execute_sql(sql)
        return result.get("records", [])

    async def update_returning(self, storage_type: StorageType, table: str,
                               conditions: Dict, data: Dict,
                               returning: List[str]) -> List[Dict]:
        set_clause = ", ".join(f"{k} = '{v}'" if isinstance(v, str) else f"{k} = {v}" for k, v in data.items())
        conds = " AND ".join(
            f"{k} = '{v}'" if isinstance(v, str) else f"{k} = {v}" for k, v in conditions.items()
        )
        ret = ", ".join(returning)
        sql = f"UPDATE {table} SET {set_clause} WHERE {conds} RETURNING {ret}"
        result = await self.execute_sql(sql)
        return result.get("records", [])

    async def delete_returning(self, storage_type: StorageType, table: str,
                               conditions: Dict, returning: List[str]) -> List[Dict]:
        conds = " AND ".join(
            f"{k} = '{v}'" if isinstance(v, str) else f"{k} = {v}" for k, v in conditions.items()
        )
        ret = ", ".join(returning)
        sql = f"DELETE FROM {table} WHERE {conds} RETURNING {ret}"
        result = await self.execute_sql(sql)
        return result.get("records", [])

    async def select_grouped(
        self, storage_type: StorageType, table: str,
        columns: List[str] = None,
        conditions: Optional[Dict] = None,
        group_by: Optional[List[str]] = None,
        having: Optional[Dict] = None,
        distinct: bool = False,
        order_by: Optional[List[str]] = None,
        limit: Optional[int] = None,
        offset: Optional[int] = None,
    ) -> List[Dict]:
        cols = "*"
        if columns:
            cols = ", ".join(columns)
        sql = "SELECT "
        if distinct:
            sql += "DISTINCT "
        sql += f"{cols} FROM {table}"
        if conditions:
            conds = " AND ".join(
                f"{k} = '{v}'" if isinstance(v, str) else f"{k} = {v}" for k, v in conditions.items()
            )
            sql += f" WHERE {conds}"
        if group_by:
            sql += f" GROUP BY {', '.join(group_by)}"
        if having:
            having_conds = " AND ".join(
                f"{k} = '{v}'" if isinstance(v, str) else f"{k} = {v}" for k, v in having.items()
            )
            sql += f" HAVING {having_conds}"
        if order_by:
            sql += f" ORDER BY {', '.join(order_by)}"
        if limit:
            sql += f" LIMIT {limit}"
        if offset:
            sql += f" OFFSET {offset}"
        result = await self.execute_sql(sql)
        return result.get("records", [])

    async def add_foreign_key(self, storage_type: StorageType, table: str,
                              name: str, column: str,
                              references_table: str, references_column: str,
                              on_delete: str = "Restrict",
                              on_update: str = "Restrict") -> Dict:
        constraint = {
            "name": name,
            "constraint_type": "ForeignKey",
            "fields": [column],
            "definition": {
                "references_table": references_table,
                "references_field": references_column,
                "on_delete": on_delete,
                "on_update": on_update,
            }
        }
        return await self.add_constraint(storage_type, table, constraint)

    async def drop_foreign_key(self, storage_type: StorageType, table: str,
                               constraint_name: str) -> Dict:
        return await self.drop_constraint(storage_type, table, constraint_name)

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