package com.primusdb.jdbc;

import java.sql.*;
import okhttp3.*;
import com.google.gson.*;
import java.io.IOException;

/**
 * JDBC Connection implementation for PrimusDB
 */
public class PrimusDBConnection implements java.sql.Connection {

    private final String host;
    private final int port;
    private final String database;
    private final String username;
    private final String password;
    private final OkHttpClient httpClient;
    private final Gson gson;
    private boolean closed = false;
    private boolean autoCommit = true;

    public PrimusDBConnection(String host, int port, String database, String username, String password) {
        this.host = host;
        this.port = port;
        this.database = database;
        this.username = username;
        this.password = password;
        this.httpClient = new OkHttpClient.Builder().build();
        this.gson = new Gson();
    }

    public String getBaseUrl() {
        return "http://" + host + ":" + port;
    }

    public OkHttpClient getHttpClient() {
        return httpClient;
    }

    @Override
    public Statement createStatement() throws SQLException {
        checkClosed();
        return new PrimusDBStatement(this);
    }

    @Override
    public PreparedStatement prepareStatement(String sql) throws SQLException {
        throw new SQLFeatureNotSupportedException("Prepared statements not yet implemented");
    }

    @Override
    public CallableStatement prepareCall(String sql) throws SQLException {
        throw new SQLFeatureNotSupportedException("Callable statements not supported");
    }

    @Override
    public String nativeSQL(String sql) throws SQLException {
        return sql; // No transformation needed
    }

    @Override
    public void setAutoCommit(boolean autoCommit) throws SQLException {
        this.autoCommit = autoCommit;
    }

    @Override
    public boolean getAutoCommit() throws SQLException {
        return autoCommit;
    }

    @Override
    public void commit() throws SQLException {
        // Transaction commit - simplified
    }

    @Override
    public void rollback() throws SQLException {
        // Transaction rollback - simplified
    }

    @Override
    public void close() throws SQLException {
        closed = true;
    }

    @Override
    public boolean isClosed() throws SQLException {
        return closed;
    }

    @Override
    public DatabaseMetaData getMetaData() throws SQLException {
        return new PrimusDBDatabaseMetaData(this);
    }

    @Override
    public void setReadOnly(boolean readOnly) throws SQLException {
        if (readOnly) {
            throw new SQLFeatureNotSupportedException("Read-only mode not supported");
        }
    }

    @Override
    public boolean isReadOnly() throws SQLException {
        return false;
    }

    @Override
    public void setCatalog(String catalog) throws SQLException {
        // Catalog not supported
    }

    @Override
    public String getCatalog() throws SQLException {
        return database;
    }

    @Override
    public void setTransactionIsolation(int level) throws SQLException {
        if (level != TRANSACTION_NONE) {
            throw new SQLFeatureNotSupportedException("Transaction isolation not supported");
        }
    }

    @Override
    public int getTransactionIsolation() throws SQLException {
        return TRANSACTION_NONE;
    }

    // Minimal implementations for other methods
    private void checkClosed() throws SQLException {
        if (closed) {
            throw new SQLException("Connection is closed");
        }
    }

    @Override public SQLWarning getWarnings() throws SQLException { return null; }
    @Override public void clearWarnings() throws SQLException {}
    @Override public Statement createStatement(int resultSetType, int resultSetConcurrency) throws SQLException { return createStatement(); }
    @Override public PreparedStatement prepareStatement(String sql, int resultSetType, int resultSetConcurrency) throws SQLException { return prepareStatement(sql); }
    @Override public CallableStatement prepareCall(String sql, int resultSetType, int resultSetConcurrency) throws SQLException { return prepareCall(sql); }
    @Override public java.util.Map<String, Class<?>> getTypeMap() throws SQLException { return new java.util.HashMap<>(); }
    @Override public void setTypeMap(java.util.Map<String, Class<?>> map) throws SQLException {}
    @Override public void setHoldability(int holdability) throws SQLException {}
    @Override public int getHoldability() throws SQLException { return ResultSet.CLOSE_CURSORS_AT_COMMIT; }
    @Override public Savepoint setSavepoint() throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public Savepoint setSavepoint(String name) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void rollback(Savepoint savepoint) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void releaseSavepoint(Savepoint savepoint) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public Statement createStatement(int resultSetType, int resultSetConcurrency, int resultSetHoldability) throws SQLException { return createStatement(); }
    @Override public PreparedStatement prepareStatement(String sql, int resultSetType, int resultSetConcurrency, int resultSetHoldability) throws SQLException { return prepareStatement(sql); }
    @Override public CallableStatement prepareCall(String sql, int resultSetType, int resultSetConcurrency, int resultSetHoldability) throws SQLException { return prepareCall(sql); }
    @Override public PreparedStatement prepareStatement(String sql, int autoGeneratedKeys) throws SQLException { return prepareStatement(sql); }
    @Override public PreparedStatement prepareStatement(String sql, int[] columnIndexes) throws SQLException { return prepareStatement(sql); }
    @Override public PreparedStatement prepareStatement(String sql, String[] columnNames) throws SQLException { return prepareStatement(sql); }
    @Override public Clob createClob() throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public Blob createBlob() throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public NClob createNClob() throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public SQLXML createSQLXML() throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public boolean isValid(int timeout) throws SQLException { return !closed; }
    @Override public void setClientInfo(String name, String value) throws SQLClientInfoException {}
    @Override public void setClientInfo(Properties properties) throws SQLClientInfoException {}
    @Override public String getClientInfo(String name) throws SQLException { return null; }
    @Override public Properties getClientInfo() throws SQLException { return new Properties(); }
    @Override public Array createArrayOf(String typeName, Object[] elements) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public Struct createStruct(String typeName, Object[] attributes) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void abort(java.util.concurrent.Executor executor) throws SQLException { close(); }
    @Override public void setNetworkTimeout(java.util.concurrent.Executor executor, int milliseconds) throws SQLException {}
    @Override public int getNetworkTimeout() throws SQLException { return 0; }
    @Override public <T> T unwrap(Class<T> iface) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public boolean isWrapperFor(Class<?> iface) throws SQLException { return false; }
}