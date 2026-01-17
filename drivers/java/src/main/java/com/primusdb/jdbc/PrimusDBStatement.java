package com.primusdb.jdbc;

import java.sql.*;
import okhttp3.*;
import com.google.gson.*;
import java.io.IOException;
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

        if (sql.toUpperCase().startsWith("INSERT")) {
            // Simple INSERT parsing
            String table = extractTableName(sql);
            String values = extractValues(sql);

            try {
                String url = connection.getBaseUrl() + "/api/v1/query";
                String jsonBody = gson.toJson(new QueryRequest("document", "Create", table, null, values, null, null));

                RequestBody body = RequestBody.create(jsonBody, MediaType.parse("application/json"));
                Request request = new Request.Builder()
                    .url(url)
                    .post(body)
                    .build();

                try (Response response = httpClient.newCall(request).execute()) {
                    if (response.isSuccessful()) {
                        String responseBody = response.body().string();
                        JsonObject result = gson.fromJson(responseBody, JsonObject.class);
                        return result.has("count") ? result.get("count").getAsInt() : 1;
                    } else {
                        throw new SQLException("Insert failed: " + response.message());
                    }
                }
            } catch (IOException e) {
                throw new SQLException("Network error: " + e.getMessage(), e);
            }
        }

        throw new SQLFeatureNotSupportedException("UPDATE/DELETE not yet implemented");
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
    @Override public Connection getConnection() throws SQLException { return connection; }

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