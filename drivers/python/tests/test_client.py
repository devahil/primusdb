import json
import pytest
from aioresponses import aioresponses
from primusdb import (
    PrimusDBClient,
    StorageType,
    ConnectionConfig,
    Collection,
    connect,
)


# ==================== StorageType ====================

class TestStorageType:
    def test_values(self):
        assert StorageType.COLUMNAR.value == "columnar"
        assert StorageType.VECTOR.value == "vector"
        assert StorageType.DOCUMENT.value == "document"
        assert StorageType.RELATIONAL.value == "relational"

    def test_members(self):
        assert len(StorageType) == 4


# ==================== ConnectionConfig ====================

class TestConnectionConfig:
    def test_defaults(self):
        config = ConnectionConfig()
        assert config.host == "localhost"
        assert config.port == 8080
        assert config.timeout == 30.0
        assert config.max_connections == 10

    def test_custom(self):
        config = ConnectionConfig(host="10.0.0.1", port=9090, timeout=15.0, max_connections=5)
        assert config.host == "10.0.0.1"
        assert config.port == 9090
        assert config.timeout == 15.0
        assert config.max_connections == 5

    def test_partial(self):
        config = ConnectionConfig(host="example.com")
        assert config.host == "example.com"
        assert config.port == 8080


# ==================== PrimusDBClient - Lifecycle ====================

class TestPrimusDBClientLifecycle:
    def test_init_default_config(self):
        client = PrimusDBClient()
        assert client.config.host == "localhost"
        assert client.config.port == 8080
        assert not client._connected
        assert client._session is None

    def test_init_custom_config(self):
        config = ConnectionConfig(host="10.0.0.1", port=9090)
        client = PrimusDBClient(config)
        assert client.config.host == "10.0.0.1"
        assert client.config.port == 9090

    @pytest.mark.asyncio
    async def test_connect(self):
        client = PrimusDBClient()
        await client.connect()
        assert client._connected
        assert client._session is not None
        await client.close()

    @pytest.mark.asyncio
    async def test_close(self):
        client = PrimusDBClient()
        await client.connect()
        await client.close()
        assert not client._connected

    @pytest.mark.asyncio
    async def test_async_context_manager(self):
        async with PrimusDBClient() as client:
            assert client._connected
            assert client._session is not None
        assert not client._connected

    @pytest.mark.asyncio
    async def test_connect_fails_without_aiohttp(self, monkeypatch):
        original_import = __builtins__["__import__"]
        def mock_import(name, *args, **kwargs):
            if name == "aiohttp":
                raise ImportError("No module named aiohttp")
            return original_import(name, *args, **kwargs)
        monkeypatch.setattr("builtins.__import__", mock_import)
        client = PrimusDBClient()
        with pytest.raises(ImportError, match="aiohttp is required"):
            await client.connect()


# ==================== PrimusDBClient - _request ====================

class TestRequest:
    @pytest.mark.asyncio
    async def test_request_not_connected(self):
        client = PrimusDBClient()
        with pytest.raises(ConnectionError, match="Not connected"):
            await client._request("GET", "test")

    @pytest.mark.asyncio
    async def test_request_success(self):
        with aioresponses() as mocked:
            mocked.post(
                "http://localhost:8080/api/v1/test",
                payload={"success": True, "data": {"key": "value"}},
            )
            async with PrimusDBClient() as client:
                result = await client._request("POST", "test", {"foo": "bar"})
                assert result == {"key": "value"}

    @pytest.mark.asyncio
    async def test_request_server_error(self):
        with aioresponses() as mocked:
            mocked.post(
                "http://localhost:8080/api/v1/test",
                payload={"success": False, "error": "Something broke"},
            )
            async with PrimusDBClient() as client:
                with pytest.raises(RuntimeError, match="Something broke"):
                    await client._request("POST", "test")

    @pytest.mark.asyncio
    async def test_request_unknown_error(self):
        with aioresponses() as mocked:
            mocked.post(
                "http://localhost:8080/api/v1/test",
                payload={"success": False},
            )
            async with PrimusDBClient() as client:
                with pytest.raises(RuntimeError, match="Unknown error"):
                    await client._request("POST", "test")


# ==================== PrimusDBClient - CRUD ====================

class TestCRUD:
    BASE = "http://localhost:8080/api/v1"

    @pytest.mark.asyncio
    async def test_create_table(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/table/relational/users"
            mocked.post(url, payload={"success": True, "data": None})
            async with PrimusDBClient() as client:
                await client.create_table(StorageType.RELATIONAL, "users", {"id": "int"})

    @pytest.mark.asyncio
    async def test_insert(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/crud/relational/users"
            mocked.post(url, payload={"success": True, "data": {"count": 1}})
            async with PrimusDBClient() as client:
                count = await client.insert(StorageType.RELATIONAL, "users", {"name": "Alice"})
                assert count == 1

    @pytest.mark.asyncio
    async def test_select(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/crud/columnar/items"
            mocked.get(url, payload={"success": True, "data": [{"id": 1}]})
            async with PrimusDBClient() as client:
                rows = await client.select(StorageType.COLUMNAR, "items")
                assert rows == [{"id": 1}]

    @pytest.mark.asyncio
    async def test_update(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/crud/document/docs"
            mocked.put(url, payload={"success": True, "data": {"count": 2}})
            async with PrimusDBClient() as client:
                count = await client.update(
                    StorageType.DOCUMENT, "docs",
                    {"status": "draft"}, {"status": "published"},
                )
                assert count == 2

    @pytest.mark.asyncio
    async def test_delete(self):
        with aioresponses() as mocked:
            import re
            mocked.delete(re.compile(r".*/crud/vector/vectors.*"), payload={"success": True, "data": {"count": 3}})
            async with PrimusDBClient() as client:
                count = await client.delete(
                    StorageType.VECTOR, "vectors",
                    {"category": "test"},
                )
                assert count == 3

    @pytest.mark.asyncio
    async def test_select_with_conditions(self):
        with aioresponses() as mocked:
            import re
            mocked.get(re.compile(r".*/crud/relational/users.*"), payload={"success": True, "data": [{"id": 1}]})
            async with PrimusDBClient() as client:
                rows = await client.select(
                    StorageType.RELATIONAL, "users",
                    conditions={"name": "Alice"},
                    limit=10, offset=0,
                )
                assert rows == [{"id": 1}]

    @pytest.mark.asyncio
    async def test_truncate_table(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/crud/relational/users/truncate"
            mocked.post(url, payload={"success": True, "data": {"truncated": True}})
            async with PrimusDBClient() as client:
                result = await client.truncate_table(StorageType.RELATIONAL, "users", cascade=True)
                assert result["truncated"] is True


# ==================== PrimusDBClient - Advanced Operations ====================

class TestAdvanced:
    BASE = "http://localhost:8080/api/v1"

    @pytest.mark.asyncio
    async def test_analyze(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/advanced/analyze/columnar/sales"
            mocked.post(url, payload={"success": True, "data": {"mean": 42.0}})
            async with PrimusDBClient() as client:
                result = await client.analyze(StorageType.COLUMNAR, "sales")
                assert result == {"mean": 42.0}

    @pytest.mark.asyncio
    async def test_predict(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/advanced/predict/columnar/sales"
            mocked.post(url, payload={"success": True, "data": {"prediction": 100}})
            async with PrimusDBClient() as client:
                result = await client.predict(
                    StorageType.COLUMNAR, "sales",
                    {"features": [1, 2, 3]},
                )
                assert result == {"prediction": 100}

    @pytest.mark.asyncio
    async def test_vector_search(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/advanced/vector-search/embeddings"
            mocked.post(url, payload={"success": True, "data": [{"id": 1, "score": 0.95}]})
            async with PrimusDBClient() as client:
                results = await client.vector_search("embeddings", [0.1, 0.2, 0.3], limit=5)
                assert results == [{"id": 1, "score": 0.95}]

    @pytest.mark.asyncio
    async def test_cluster(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/advanced/cluster/document/docs"
            mocked.post(url, payload={"success": True, "data": {"clusters": 3}})
            async with PrimusDBClient() as client:
                result = await client.cluster(StorageType.DOCUMENT, "docs")
                assert result == {"clusters": 3}

    @pytest.mark.asyncio
    @pytest.mark.parametrize("method, endpoint", [
        ("cluster_status", "cluster/status"),
        ("cluster_nodes", "cluster/nodes"),
        ("cluster_metrics", "cluster/metrics"),
    ])
    async def test_cluster_gateway(self, method, endpoint):
        with aioresponses() as mocked:
            url = f"{self.BASE}/{endpoint}"
            mocked.get(url, payload={"success": True, "data": {}})
            async with PrimusDBClient() as client:
                result = await getattr(client, method)()
                assert result == {}

    @pytest.mark.asyncio
    async def test_route_request(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/cluster/route"
            mocked.post(url, payload={"success": True, "data": {"node": "n1"}})
            async with PrimusDBClient() as client:
                result = await client.route_request(shard_key="abc")
                assert result == {"node": "n1"}

    @pytest.mark.asyncio
    @pytest.mark.parametrize("method, endpoint", [
        ("federation_status", "federation/status"),
        ("federation_clusters", "federation/clusters"),
        ("federation_domains", "federation/domains"),
        ("federation_metrics", "federation/metrics"),
    ])
    async def test_federation_getters(self, method, endpoint):
        with aioresponses() as mocked:
            url = f"{self.BASE}/{endpoint}"
            mocked.get(url, payload={"success": True, "data": {}})
            async with PrimusDBClient() as client:
                result = await getattr(client, method)()
                assert result == {}

    @pytest.mark.asyncio
    async def test_create_data_domain(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/federation/domains"
            mocked.post(url, payload={"success": True, "data": {"name": "domain1"}})
            async with PrimusDBClient() as client:
                result = await client.create_data_domain("domain1", storage_types=["columnar"])
                assert result == {"name": "domain1"}


# ==================== PrimusDBClient - DDL / Sequences / Views / Triggers / Info Schema ====================

class TestDDL:
    BASE = "http://localhost:8080/api/v1"

    @pytest.mark.asyncio
    async def test_add_column(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/ddl/relational/users/column/add"
            mocked.post(url, payload={"success": True, "data": {}})
            async with PrimusDBClient() as client:
                await client.add_column(StorageType.RELATIONAL, "users", {"name": "TEXT"})

    @pytest.mark.asyncio
    async def test_drop_column(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/ddl/relational/users/column/age"
            mocked.delete(url, payload={"success": True, "data": {}})
            async with PrimusDBClient() as client:
                await client.drop_column(StorageType.RELATIONAL, "users", "age")

    @pytest.mark.asyncio
    async def test_modify_column(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/ddl/relational/users/column"
            mocked.put(url, payload={"success": True, "data": {}})
            async with PrimusDBClient() as client:
                await client.modify_column(StorageType.RELATIONAL, "users", {"name": "VARCHAR(100)"})

    @pytest.mark.asyncio
    async def test_create_drop_sequence(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/sequence/relational"
            mocked.post(url, payload={"success": True, "data": {"name": "seq1"}})
            async with PrimusDBClient() as client:
                result = await client.create_sequence(StorageType.RELATIONAL, {"name": "seq1"})
                assert result == {"name": "seq1"}

        with aioresponses() as mocked:
            url = f"{self.BASE}/sequence/relational/seq1"
            mocked.delete(url, payload={"success": True, "data": {}})
            async with PrimusDBClient() as client:
                await client.drop_sequence(StorageType.RELATIONAL, "seq1")

    @pytest.mark.asyncio
    async def test_nextval_currval_setval(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/sequence/relational/seq1/nextval"
            mocked.post(url, payload={"success": True, "data": {"nextval": 1}})
            async with PrimusDBClient() as client:
                result = await client.nextval(StorageType.RELATIONAL, "seq1")
                assert result == {"nextval": 1}

        with aioresponses() as mocked:
            url = f"{self.BASE}/sequence/relational/seq1/currval"
            mocked.get(url, payload={"success": True, "data": {"currval": 1}})
            async with PrimusDBClient() as client:
                result = await client.currval(StorageType.RELATIONAL, "seq1")
                assert result == {"currval": 1}

        with aioresponses() as mocked:
            url = f"{self.BASE}/sequence/relational/seq1/setval"
            mocked.post(url, payload={"success": True, "data": {"value": 100}})
            async with PrimusDBClient() as client:
                result = await client.setval(StorageType.RELATIONAL, "seq1", 100)
                assert result == {"value": 100}

    @pytest.mark.asyncio
    async def test_create_drop_view(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/view/relational"
            mocked.post(url, payload={"success": True, "data": {"name": "v1"}})
            async with PrimusDBClient() as client:
                result = await client.create_view(StorageType.RELATIONAL, {"name": "v1"})
                assert result == {"name": "v1"}

        with aioresponses() as mocked:
            url = f"{self.BASE}/view/relational/v1"
            mocked.delete(url, payload={"success": True, "data": {}})
            async with PrimusDBClient() as client:
                await client.drop_view(StorageType.RELATIONAL, "v1")

    @pytest.mark.asyncio
    async def test_create_drop_trigger(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/trigger/relational/users"
            mocked.post(url, payload={"success": True, "data": {"name": "trg1"}})
            async with PrimusDBClient() as client:
                result = await client.create_trigger(StorageType.RELATIONAL, "users", {"name": "trg1"})
                assert result == {"name": "trg1"}

        with aioresponses() as mocked:
            url = f"{self.BASE}/trigger/relational/users/trg1"
            mocked.delete(url, payload={"success": True, "data": {}})
            async with PrimusDBClient() as client:
                await client.drop_trigger(StorageType.RELATIONAL, "users", "trg1")

    @pytest.mark.asyncio
    async def test_info_schema(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/info-schema/relational/tables"
            mocked.get(url, payload={"success": True, "data": {"tables": ["users"]}})
            async with PrimusDBClient() as client:
                result = await client.info_schema_tables(StorageType.RELATIONAL)
                assert result == {"tables": ["users"]}

        with aioresponses() as mocked:
            url = f"{self.BASE}/info-schema/relational/users/columns"
            mocked.get(url, payload={"success": True, "data": {"columns": ["id", "name"]}})
            async with PrimusDBClient() as client:
                result = await client.info_schema_columns(StorageType.RELATIONAL, "users")
                assert result == {"columns": ["id", "name"]}

        with aioresponses() as mocked:
            url = f"{self.BASE}/info-schema/relational/users/constraints"
            mocked.get(url, payload={"success": True, "data": {"constraints": ["pk_users"]}})
            async with PrimusDBClient() as client:
                result = await client.info_schema_constraints(StorageType.RELATIONAL, "users")
                assert result == {"constraints": ["pk_users"]}

    @pytest.mark.asyncio
    async def test_add_drop_foreign_key(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/ddl/relational/orders/constraint"
            mocked.post(url, payload={"success": True, "data": {"name": "fk_user"}})
            async with PrimusDBClient() as client:
                result = await client.add_foreign_key(
                    StorageType.RELATIONAL, "orders",
                    "fk_user", "user_id",
                    "users", "id",
                )
                assert result == {"name": "fk_user"}

        with aioresponses() as mocked:
            url = f"{self.BASE}/ddl/relational/orders/constraint/fk_user"
            mocked.delete(url, payload={"success": True, "data": {}})
            async with PrimusDBClient() as client:
                await client.drop_foreign_key(StorageType.RELATIONAL, "orders", "fk_user")

    @pytest.mark.asyncio
    async def test_rename_table(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/ddl/relational/old_name/rename"
            mocked.post(url, payload={"success": True, "data": {}})
            async with PrimusDBClient() as client:
                await client.rename_table(StorageType.RELATIONAL, "old_name", "new_name")


# ==================== PrimusDBClient - SQL / UQL ====================

class TestSQL:
    BASE = "http://localhost:8080/api/v1"

    @pytest.mark.asyncio
    async def test_execute_sql(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/uql"
            mocked.post(url, payload={"success": True, "data": {"records": [{"id": 1}]}})
            async with PrimusDBClient() as client:
                result = await client.execute_sql("SELECT * FROM users")
                assert result == {"records": [{"id": 1}]}

    @pytest.mark.asyncio
    async def test_insert_returning(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/uql"
            mocked.post(url, payload={"success": True, "data": {"records": [{"id": 1}]}})
            async with PrimusDBClient() as client:
                result = await client.insert_returning(
                    StorageType.RELATIONAL, "users",
                    {"name": "Bob"}, returning=["id"],
                )
                assert result == [{"id": 1}]

    @pytest.mark.asyncio
    async def test_update_returning(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/uql"
            mocked.post(url, payload={"success": True, "data": {"records": [{"id": 1}]}})
            async with PrimusDBClient() as client:
                result = await client.update_returning(
                    StorageType.RELATIONAL, "users",
                    {"id": 1}, {"name": "Bob"}, returning=["id"],
                )
                assert result == [{"id": 1}]

    @pytest.mark.asyncio
    async def test_delete_returning(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/uql"
            mocked.post(url, payload={"success": True, "data": {"records": [{"id": 1}]}})
            async with PrimusDBClient() as client:
                result = await client.delete_returning(
                    StorageType.RELATIONAL, "users",
                    {"id": 1}, returning=["id"],
                )
                assert result == [{"id": 1}]

    @pytest.mark.asyncio
    async def test_select_grouped(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/uql"
            mocked.post(url, payload={"success": True, "data": {"records": [{"dept": "eng", "count": 5}]}})
            async with PrimusDBClient() as client:
                result = await client.select_grouped(
                    StorageType.RELATIONAL, "employees",
                    columns=["dept", "COUNT(*) as count"],
                    conditions={"status": "active"},
                    group_by=["dept"],
                    order_by=["count DESC"],
                    limit=10,
                )
                assert result == [{"dept": "eng", "count": 5}]


# ==================== Collection ====================

class TestCollection:
    BASE = "http://localhost:8080/api/v1"

    @pytest.mark.asyncio
    async def test_collection_init(self):
        async with PrimusDBClient() as client:
            coll = Collection(client, StorageType.RELATIONAL, "users")
            assert coll.client is client
            assert coll.storage_type == StorageType.RELATIONAL
            assert coll.name == "users"

    @pytest.mark.asyncio
    async def test_collection_crud(self):
        with aioresponses() as mocked:
            import re
            mocked.get(re.compile(r".*/crud/relational/users.*"), payload={"success": True, "data": []})

            async with PrimusDBClient() as client:
                coll = Collection(client, StorageType.RELATIONAL, "users")

                count = await coll.count()
                assert count == 0


# ==================== connect() convenience ====================

class TestConnect:
    @pytest.mark.asyncio
    async def test_connect(self):
        client = await connect("10.0.0.1", 9090)
        assert client.config.host == "10.0.0.1"
        assert client.config.port == 9090
        assert client._connected
        await client.close()


# ==================== Error handling ====================

class TestErrors:
    BASE = "http://localhost:8080/api/v1"

    @pytest.mark.asyncio
    async def test_not_connected_on_operation(self):
        client = PrimusDBClient()
        with pytest.raises(ConnectionError):
            await client.create_table(StorageType.RELATIONAL, "t", {})

    @pytest.mark.asyncio
    async def test_insert_returns_zero_on_empty(self):
        with aioresponses() as mocked:
            url = f"{self.BASE}/crud/relational/users"
            mocked.post(url, payload={"success": True, "data": {}})
            async with PrimusDBClient() as client:
                count = await client.insert(StorageType.RELATIONAL, "users", {"x": 1})
                assert count == 0
