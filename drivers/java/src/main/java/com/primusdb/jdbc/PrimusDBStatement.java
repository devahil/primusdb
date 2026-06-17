package com.primusdb.jdbc;

import java.sql.*;
import okhttp3.*;
import com.google.gson.*;
import java.io.IOException;
import java.net.URLEncoder;
import java.util.List;
import java.util.ArrayList;

/**
 * JDBC Statement implementation for PrimusDB
 */
public class PrimusDBStatement implements Statement {

    private final PrimusDBConnection connection;
    private final OkHttpClient httpClient;
    private final Gson gson;
    private boolean closed = false;
    private ResultSet currentResultSet;

    public PrimusDBStatement(PrimusDBConnection connection) {
        this.connection = connection;
        this.httpClient = connection.getHttpClient();
        this.gson = new Gson();
    }

    @Override
    public ResultSet executeQuery(String sql) throws SQLException {
        checkClosed();

        // Parse simple SQL-like queries for PrimusDB
        if (sql.toUpperCase().startsWith("SELECT")) {
            // Simple parsing for SELECT * FROM table WHERE conditions
            String table = extractTableName(sql);
            String conditions = extractConditions(sql);

            try {
                // Make HTTP request to PrimusDB
                String url = connection.getBaseUrl() + "/api/v1/crud/document/" + table;
                if (conditions != null) {
                    url += "?conditions=" + java.net.URLEncoder.encode(conditions, "UTF-8");
                }

                Request request = new Request.Builder()
                    .url(url)
                    .get()
                    .build();

                try (Response response = httpClient.newCall(request).execute()) {
                    if (response.isSuccessful()) {
                        String responseBody = response.body().string();
                        JsonArray results = gson.fromJson(responseBody, JsonArray.class);
                        return new PrimusDBResultSet(results);
                    } else {
                        throw new SQLException("Query failed: " + response.message());
                    }
                }
            } catch (IOException e) {
                throw new SQLException("Network error: " + e.getMessage(), e);
            }
        }

        throw new SQLFeatureNotSupportedException("Complex SQL queries not yet supported");
    }

    @Override
    public int executeUpdate(String sql) throws SQLException {
        checkClosed();
        String upperSql = sql.trim().toUpperCase();

        try {
            if (upperSql.startsWith("INSERT")) {
                String table = extractTableName(sql);
                String values = extractValues(sql);
                String url = connection.getBaseUrl() + "/api/v1/query";
                String jsonBody = gson.toJson(new QueryRequest("document", "Create", table, null, values, null, null));
                RequestBody body = RequestBody.create(jsonBody, MediaType.parse("application/json"));
                Request request = new Request.Builder().url(url).post(body).build();
                try (Response response = httpClient.newCall(request).execute()) {
                    if (response.isSuccessful()) {
                        String responseBody = response.body().string();
                        JsonObject result = gson.fromJson(responseBody, JsonObject.class);
                        return result.has("count") ? result.get("count").getAsInt() : 1;
                    } else {
                        throw new SQLException("Insert failed: " + response.message());
                    }
                }
            } else if (upperSql.startsWith("UPDATE")) {
                String table = extractTableName(sql);
                String url = connection.getBaseUrl() + "/api/v1/query";
                JsonObject data = new JsonObject();
                data.addProperty("storage_type", "document");
                data.addProperty("operation", "Update");
                data.addProperty("table", table);
                String jsonBody = gson.toJson(data);
                RequestBody body = RequestBody.create(jsonBody, MediaType.parse("application/json"));
                Request request = new Request.Builder().url(url).post(body).build();
                try (Response response = httpClient.newCall(request).execute()) {
                    if (response.isSuccessful()) return 1;
                    else throw new SQLException("Update failed: " + response.message());
                }
            } else if (upperSql.startsWith("DELETE")) {
                String table = extractTableName(sql);
                String url = connection.getBaseUrl() + "/api/v1/query";
                JsonObject data = new JsonObject();
                data.addProperty("storage_type", "document");
                data.addProperty("operation", "Delete");
                data.addProperty("table", table);
                String jsonBody = gson.toJson(data);
                RequestBody body = RequestBody.create(jsonBody, MediaType.parse("application/json"));
                Request request = new Request.Builder().url(url).post(body).build();
                try (Response response = httpClient.newCall(request).execute()) {
                    if (response.isSuccessful()) return 1;
                    else throw new SQLException("Delete failed: " + response.message());
                }
            } else if (upperSql.startsWith("ALTER TABLE")) {
                String table = extractAlterTableName(sql);
                if (upperSql.contains("ADD COLUMN") || upperSql.contains("ADD")) {
                    String columnDef = extractAfterKeyword(sql, "ADD COLUMN", "ADD");
                    JsonObject field = new JsonObject();
                    field.addProperty("name", extractColumnName(columnDef));
                    field.addProperty("type", extractColumnType(columnDef));
                    String url = connection.getBaseUrl() + "/api/v1/ddl/relational/" + table + "/column/add";
                    RequestBody body = RequestBody.create(field.toString(), MediaType.parse("application/json"));
                    Request request = new Request.Builder().url(url).post(body).build();
                    try (Response response = httpClient.newCall(request).execute()) {
                        if (response.isSuccessful()) return 1;
                        else throw new SQLException("Add column failed: " + response.message());
                    }
                } else if (upperSql.contains("DROP COLUMN") || upperSql.contains("DROP")) {
                    String columnName = extractColumnName(extractAfterKeyword(sql, "DROP COLUMN", "DROP"));
                    String url = connection.getBaseUrl() + "/api/v1/ddl/relational/" + table + "/column/" + columnName;
                    Request request = new Request.Builder().url(url).delete().build();
                    try (Response response = httpClient.newCall(request).execute()) {
                        if (response.isSuccessful()) return 1;
                        else throw new SQLException("Drop column failed: " + response.message());
                    }
                } else if (upperSql.contains("MODIFY COLUMN") || upperSql.contains("MODIFY")) {
                    String columnDef = extractAfterKeyword(sql, "MODIFY COLUMN", "MODIFY");
                    JsonObject field = new JsonObject();
                    field.addProperty("name", extractColumnName(columnDef));
                    field.addProperty("type", extractColumnType(columnDef));
                    String url = connection.getBaseUrl() + "/api/v1/ddl/relational/" + table + "/column";
                    RequestBody body = RequestBody.create(field.toString(), MediaType.parse("application/json"));
                    Request request = new Request.Builder().url(url).put(body).build();
                    try (Response response = httpClient.newCall(request).execute()) {
                        if (response.isSuccessful()) return 1;
                        else throw new SQLException("Modify column failed: " + response.message());
                    }
                } else if (upperSql.contains("RENAME TO") || upperSql.contains("RENAME")) {
                    String newName = extractAfterKeyword(sql, "RENAME TO", "RENAME").trim();
                    JsonObject data = new JsonObject();
                    data.addProperty("new_name", newName);
                    String url = connection.getBaseUrl() + "/api/v1/ddl/relational/" + table + "/rename";
                    RequestBody body = RequestBody.create(data.toString(), MediaType.parse("application/json"));
                    Request request = new Request.Builder().url(url).post(body).build();
                    try (Response response = httpClient.newCall(request).execute()) {
                        if (response.isSuccessful()) return 1;
                        else throw new SQLException("Rename table failed: " + response.message());
                    }
                } else {
                    throw new SQLFeatureNotSupportedException("ALTER TABLE operation not supported: " + sql);
                }
            } else if (upperSql.startsWith("CREATE SEQUENCE")) {
                String seqName = extractAfterKeyword(sql, "CREATE SEQUENCE", "SEQUENCE").trim().split("\\s+")[0];
                JsonObject sequence = new JsonObject();
                sequence.addProperty("name", seqName);
                String url = connection.getBaseUrl() + "/api/v1/sequence/relational";
                RequestBody body = RequestBody.create(sequence.toString(), MediaType.parse("application/json"));
                Request request = new Request.Builder().url(url).post(body).build();
                try (Response response = httpClient.newCall(request).execute()) {
                    if (response.isSuccessful()) return 1;
                    else throw new SQLException("Create sequence failed: " + response.message());
                }
            } else if (upperSql.startsWith("DROP SEQUENCE")) {
                String seqName = extractAfterKeyword(sql, "DROP SEQUENCE", "SEQUENCE").trim().split("\\s+")[0];
                String url = connection.getBaseUrl() + "/api/v1/sequence/relational/" + seqName;
                Request request = new Request.Builder().url(url).delete().build();
                try (Response response = httpClient.newCall(request).execute()) {
                    if (response.isSuccessful()) return 1;
                    else throw new SQLException("Drop sequence failed: " + response.message());
                }
            } else if (upperSql.startsWith("CREATE VIEW")) {
                String viewName = extractAfterKeyword(sql, "CREATE VIEW", "VIEW").trim().split("\\s+")[0];
                String queryDef = extractViewQuery(sql);
                JsonObject view = new JsonObject();
                view.addProperty("name", viewName);
                view.addProperty("query_definition", queryDef);
                String url = connection.getBaseUrl() + "/api/v1/view/relational";
                RequestBody body = RequestBody.create(view.toString(), MediaType.parse("application/json"));
                Request request = new Request.Builder().url(url).post(body).build();
                try (Response response = httpClient.newCall(request).execute()) {
                    if (response.isSuccessful()) return 1;
                    else throw new SQLException("Create view failed: " + response.message());
                }
            } else if (upperSql.startsWith("DROP VIEW")) {
                String viewName = extractAfterKeyword(sql, "DROP VIEW", "VIEW").trim().split("\\s+")[0];
                String url = connection.getBaseUrl() + "/api/v1/view/relational/" + viewName;
                Request request = new Request.Builder().url(url).delete().build();
                try (Response response = httpClient.newCall(request).execute()) {
                    if (response.isSuccessful()) return 1;
                    else throw new SQLException("Drop view failed: " + response.message());
                }
            } else if (upperSql.startsWith("CREATE TRIGGER")) {
                String[] parts = extractTriggerParts(sql);
                String trigName = parts[0];
                String trigTable = parts[1];
                JsonObject trigger = new JsonObject();
                trigger.addProperty("name", trigName);
                trigger.addProperty("table_name", trigTable);
                trigger.addProperty("timing", "AFTER");
                trigger.addProperty("event", "INSERT");
                trigger.addProperty("operation", "EXECUTE");
                String url = connection.getBaseUrl() + "/api/v1/trigger/relational/" + trigTable;
                RequestBody body = RequestBody.create(trigger.toString(), MediaType.parse("application/json"));
                Request request = new Request.Builder().url(url).post(body).build();
                try (Response response = httpClient.newCall(request).execute()) {
                    if (response.isSuccessful()) return 1;
                    else throw new SQLException("Create trigger failed: " + response.message());
                }
            } else if (upperSql.startsWith("DROP TRIGGER")) {
                String rest = upperSql.substring("DROP TRIGGER".length()).trim();
                String trigName = rest.split("\\s+")[0];
                int onIdx = rest.toUpperCase().indexOf("ON");
                String trigTable = "public";
                if (onIdx != -1) {
                    trigTable = rest.substring(onIdx + 2).trim().split("\\s+")[0];
                }
                String url = connection.getBaseUrl() + "/api/v1/trigger/relational/" + trigTable + "/" + trigName;
                Request request = new Request.Builder().url(url).delete().build();
                try (Response response = httpClient.newCall(request).execute()) {
                    if (response.isSuccessful()) return 1;
                    else throw new SQLException("Drop trigger failed: " + response.message());
                }
            } else if (upperSql.startsWith("TRUNCATE")) {
                String table = extractTableName(sql);
                boolean cascade = upperSql.contains("CASCADE");
                String url = connection.getBaseUrl() + "/api/v1/crud/relational/" + table + "/truncate";
                JsonObject body = new JsonObject();
                body.addProperty("cascade", cascade);
                RequestBody reqBody = RequestBody.create(body.toString(), MediaType.parse("application/json"));
                Request request = new Request.Builder().url(url).post(reqBody).build();
                try (Response response = httpClient.newCall(request).execute()) {
                    if (response.isSuccessful()) return 1;
                    else throw new SQLException("Truncate failed: " + response.message());
                }
            } else if (upperSql.startsWith("DROP TABLE")) {
                String table = extractTableName(sql);
                String url = connection.getBaseUrl() + "/api/v1/crud/relational/" + table;
                Request request = new Request.Builder().url(url).delete().build();
                try (Response response = httpClient.newCall(request).execute()) {
                    if (response.isSuccessful()) return 1;
                    else throw new SQLException("Drop table failed: " + response.message());
                }
            } else {
                throw new SQLFeatureNotSupportedException("SQL operation not supported: " + sql);
            }
        } catch (IOException e) {
            throw new SQLException("Network error: " + e.getMessage(), e);
        }
    }

    @Override
    public boolean execute(String sql) throws SQLException {
        executeUpdate(sql);
        return false;
    }

    @Override
    public ResultSet getResultSet() throws SQLException {
        return currentResultSet;
    }

    @Override
    public int getUpdateCount() throws SQLException {
        return -1; // Not supported
    }

    @Override
    public boolean getMoreResults() throws SQLException {
        return false;
    }

    @Override
    public void close() throws SQLException {
        closed = true;
        if (currentResultSet != null) {
            currentResultSet.close();
        }
    }

    @Override
    public boolean isClosed() throws SQLException {
        return closed;
    }

    // Helper methods
    private void checkClosed() throws SQLException {
        if (closed) {
            throw new SQLException("Statement is closed");
        }
    }

    private String extractTableName(String sql) {
        // Very simple parsing - in real implementation use proper SQL parser
        String upperSql = sql.toUpperCase();
        int fromIndex = upperSql.indexOf("FROM");
        if (fromIndex != -1) {
            int tableStart = fromIndex + 4;
            int tableEnd = sql.indexOf(" ", tableStart);
            if (tableEnd == -1) tableEnd = sql.length();
            return sql.substring(tableStart, tableEnd).trim();
        }
        return "unknown";
    }

    private String extractConditions(String sql) {
        // Simple WHERE clause extraction
        String upperSql = sql.toUpperCase();
        int whereIndex = upperSql.indexOf("WHERE");
        if (whereIndex != -1) {
            return sql.substring(whereIndex + 5).trim();
        }
        return null;
    }

    private String extractValues(String sql) {
        // Simple VALUES extraction
        String upperSql = sql.toUpperCase();
        int valuesIndex = upperSql.indexOf("VALUES");
        if (valuesIndex != -1) {
            return sql.substring(valuesIndex + 6).trim();
        }
        return "{}";
    }

    private String extractAlterTableName(String sql) {
        // ALTER TABLE table_name ...
        String upperSql = sql.toUpperCase();
        int alterIdx = upperSql.indexOf("ALTER TABLE");
        if (alterIdx != -1) {
            int tableStart = alterIdx + 11;
            while (tableStart < sql.length() && sql.charAt(tableStart) == ' ') tableStart++;
            int tableEnd = sql.indexOf(" ", tableStart);
            if (tableEnd == -1) tableEnd = sql.length();
            // Handle quoted identifiers
            String name = sql.substring(tableStart, tableEnd).trim();
            if (name.startsWith("\"") || name.startsWith("`")) name = name.substring(1, name.length() - 1);
            return name;
        }
        return "unknown";
    }

    private String extractAfterKeyword(String sql, String primaryKeyword, String fallbackKeyword) {
        String upperSql = sql.toUpperCase();
        int idx = upperSql.indexOf(primaryKeyword);
        if (idx == -1) idx = upperSql.indexOf(fallbackKeyword);
        if (idx != -1) {
            int start = idx + (primaryKeyword.equals(fallbackKeyword) || idx == upperSql.indexOf(primaryKeyword) ? primaryKeyword.length() : fallbackKeyword.length());
            return sql.substring(start).trim();
        }
        return "";
    }

    private String extractColumnName(String columnDef) {
        String trimmed = columnDef.trim().split("\\s+")[0];
        if (trimmed.startsWith("\"") || trimmed.startsWith("`")) trimmed = trimmed.substring(1, trimmed.length() - 1);
        return trimmed;
    }

    private String extractColumnType(String columnDef) {
        String[] parts = columnDef.trim().split("\\s+");
        if (parts.length > 1) return parts[1];
        return "TEXT";
    }

    private String extractViewQuery(String sql) {
        String upperSql = sql.toUpperCase();
        int asIdx = upperSql.indexOf(" AS ");
        if (asIdx != -1) {
            return sql.substring(asIdx + 4).trim();
        }
        return sql;
    }

    private String[] extractTriggerParts(String sql) {
        String upperSql = sql.toUpperCase();
        // CREATE TRIGGER name ON table ...
        int trigIdx = upperSql.indexOf("CREATE TRIGGER");
        int onIdx = upperSql.indexOf(" ON ");
        String name = "unknown";
        String table = "public";
        if (trigIdx != -1 && onIdx != -1) {
            int nameStart = trigIdx + 14;
            name = sql.substring(nameStart, onIdx).trim().split("\\s+")[0];
            int tableStart = onIdx + 4;
            int tableEnd = sql.indexOf(" ", tableStart);
            if (tableEnd == -1) tableEnd = sql.length();
            table = sql.substring(tableStart, tableEnd).trim();
        }
        return new String[]{name, table};
    }

    // Minimal implementations for other methods
    @Override public void setFetchDirection(int direction) throws SQLException {}
    @Override public int getFetchDirection() throws SQLException { return ResultSet.FETCH_FORWARD; }
    @Override public void setFetchSize(int rows) throws SQLException {}
    @Override public int getFetchSize() throws SQLException { return 0; }
    @Override public void setMaxFieldSize(int max) throws SQLException {}
    @Override public int getMaxFieldSize() throws SQLException { return 0; }
    @Override public void setMaxRows(int max) throws SQLException {}
    @Override public int getMaxRows() throws SQLException { return 0; }
    @Override public void setQueryTimeout(int seconds) throws SQLException {}
    @Override public int getQueryTimeout() throws SQLException { return 0; }
    @Override public void cancel() throws SQLException {}
    @Override public SQLWarning getWarnings() throws SQLException { return null; }
    @Override public void clearWarnings() throws SQLException {}
    @Override public void setCursorName(String name) throws SQLException {}
    @Override public void setEscapeProcessing(boolean enable) throws SQLException {}
    @Override public int getResultSetConcurrency() throws SQLException { return ResultSet.CONCUR_READ_ONLY; }
    @Override public int getResultSetType() throws SQLException { return ResultSet.TYPE_FORWARD_ONLY; }
    @Override public int getResultSetHoldability() throws SQLException { return ResultSet.CLOSE_CURSORS_AT_COMMIT; }
    @Override public boolean isPoolable() throws SQLException { return false; }
    @Override public void setPoolable(boolean poolable) throws SQLException {}
    @Override public void closeOnCompletion() throws SQLException {}
    @Override public boolean isCloseOnCompletion() throws SQLException { return false; }
    @Override public <T> T unwrap(Class<T> iface) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public boolean isWrapperFor(Class<?> iface) throws SQLException { return false; }
    @Override public java.sql.Connection getConnection() throws SQLException { return connection; }
    @Override public boolean execute(String sql, String[] columnNames) throws SQLException { return execute(sql); }
    @Override public boolean execute(String sql, int[] columnIndexes) throws SQLException { return execute(sql); }
    @Override public boolean execute(String sql, int autoGeneratedKeys) throws SQLException { return execute(sql); }
    @Override public int executeUpdate(String sql, String[] columnNames) throws SQLException { return executeUpdate(sql); }
    @Override public int executeUpdate(String sql, int[] columnIndexes) throws SQLException { return executeUpdate(sql); }
    @Override public int executeUpdate(String sql, int autoGeneratedKeys) throws SQLException { return executeUpdate(sql); }
    @Override public ResultSet getGeneratedKeys() throws SQLException { return null; }
    @Override public boolean getMoreResults(int current) throws SQLException { return false; }
    @Override public int[] executeBatch() throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void addBatch(String sql) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void clearBatch() throws SQLException { }

    // Query request helper class
    private static class QueryRequest {
        public String storage_type;
        public String operation;
        public String table;
        public String conditions;
        public String data;
        public Integer limit;
        public Integer offset;

        public QueryRequest(String storage_type, String operation, String table,
                          String conditions, String data, Integer limit, Integer offset) {
            this.storage_type = storage_type;
            this.operation = operation;
            this.table = table;
            this.conditions = conditions;
            this.data = data;
            this.limit = limit;
            this.offset = offset;
        }
    }
}