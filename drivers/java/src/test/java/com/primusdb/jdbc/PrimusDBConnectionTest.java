package com.primusdb.jdbc;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;
import java.sql.*;

class PrimusDBConnectionTest {

    private PrimusDBConnection createConnection() {
        return new PrimusDBConnection("localhost", 8080, "testdb", "user", "pass");
    }

    @Test
    void testGetBaseUrl() {
        PrimusDBConnection conn = createConnection();
        assertEquals("http://localhost:8080", conn.getBaseUrl());
    }

    @Test
    void testCreateStatement() throws SQLException {
        PrimusDBConnection conn = createConnection();
        Statement stmt = conn.createStatement();
        assertNotNull(stmt);
        assertTrue(stmt instanceof PrimusDBStatement);
    }

    @Test
    void testCloseAndIsClosed() throws SQLException {
        PrimusDBConnection conn = createConnection();
        assertFalse(conn.isClosed());
        conn.close();
        assertTrue(conn.isClosed());
    }

    @Test
    void testCreateStatementOnClosedConnection() throws SQLException {
        PrimusDBConnection conn = createConnection();
        conn.close();
        assertThrows(SQLException.class, conn::createStatement);
    }

    @Test
    void testAutoCommit() throws SQLException {
        PrimusDBConnection conn = createConnection();
        assertTrue(conn.getAutoCommit());
        conn.setAutoCommit(false);
        assertFalse(conn.getAutoCommit());
    }

    @Test
    void testCommitAndRollback() throws SQLException {
        PrimusDBConnection conn = createConnection();
        conn.commit();
        conn.rollback();
    }

    @Test
    void testCatalog() throws SQLException {
        PrimusDBConnection conn = createConnection();
        assertEquals("testdb", conn.getCatalog());
        conn.setCatalog("newdb");
        assertEquals("testdb", conn.getCatalog());
    }

    @Test
    void testSchema() throws SQLException {
        PrimusDBConnection conn = createConnection();
        assertEquals("testdb", conn.getSchema());
        conn.setSchema("newschema");
    }

    @Test
    void testReadOnly() throws SQLException {
        PrimusDBConnection conn = createConnection();
        assertFalse(conn.isReadOnly());
        assertThrows(SQLFeatureNotSupportedException.class, () -> conn.setReadOnly(true));
    }

    @Test
    void testTransactionIsolation() throws SQLException {
        PrimusDBConnection conn = createConnection();
        assertEquals(Connection.TRANSACTION_NONE, conn.getTransactionIsolation());
        conn.setTransactionIsolation(Connection.TRANSACTION_NONE);
        assertThrows(SQLFeatureNotSupportedException.class,
            () -> conn.setTransactionIsolation(Connection.TRANSACTION_READ_COMMITTED));
    }

    @Test
    void testIsValid() throws SQLException {
        PrimusDBConnection conn = createConnection();
        assertTrue(conn.isValid(5));
        conn.close();
        assertFalse(conn.isValid(5));
    }

    @Test
    void testGetMetaData() throws SQLException {
        PrimusDBConnection conn = createConnection();
        DatabaseMetaData meta = conn.getMetaData();
        assertNotNull(meta);
        assertTrue(meta instanceof PrimusDBDatabaseMetaData);
    }

    @Test
    void testWarnings() throws SQLException {
        PrimusDBConnection conn = createConnection();
        assertNull(conn.getWarnings());
        conn.clearWarnings();
    }

    @Test
    void testNativeSQL() throws SQLException {
        PrimusDBConnection conn = createConnection();
        assertEquals("SELECT 1", conn.nativeSQL("SELECT 1"));
    }

    @Test
    void testHoldability() throws SQLException {
        PrimusDBConnection conn = createConnection();
        assertEquals(ResultSet.CLOSE_CURSORS_AT_COMMIT, conn.getHoldability());
        conn.setHoldability(ResultSet.CLOSE_CURSORS_AT_COMMIT);
    }

    @Test
    void testSetSavepointThrows() throws SQLException {
        PrimusDBConnection conn = createConnection();
        assertThrows(SQLFeatureNotSupportedException.class, () -> conn.setSavepoint());
        assertThrows(SQLFeatureNotSupportedException.class, () -> conn.setSavepoint("sp"));
    }

    @Test
    void testCreateClobBlobThrows() throws SQLException {
        PrimusDBConnection conn = createConnection();
        assertThrows(SQLFeatureNotSupportedException.class, conn::createClob);
        assertThrows(SQLFeatureNotSupportedException.class, conn::createBlob);
        assertThrows(SQLFeatureNotSupportedException.class, conn::createNClob);
        assertThrows(SQLFeatureNotSupportedException.class, conn::createSQLXML);
    }

    @Test
    void testCreateArrayAndStructThrows() throws SQLException {
        PrimusDBConnection conn = createConnection();
        assertThrows(SQLFeatureNotSupportedException.class, () -> conn.createArrayOf("INTEGER", new Object[]{1}));
        assertThrows(SQLFeatureNotSupportedException.class, () -> conn.createStruct("type", new Object[]{}));
    }

    @Test
    void testTypeMap() throws SQLException {
        PrimusDBConnection conn = createConnection();
        assertNotNull(conn.getTypeMap());
        assertTrue(conn.getTypeMap().isEmpty());
        conn.setTypeMap(new java.util.HashMap<>());
    }

    @Test
    void testClientInfo() throws SQLException {
        PrimusDBConnection conn = createConnection();
        assertNull(conn.getClientInfo("app"));
        assertNotNull(conn.getClientInfo());
        conn.setClientInfo("app", "test");
        conn.setClientInfo(new java.util.Properties());
    }

    @Test
    void testNetworkTimeout() throws SQLException {
        PrimusDBConnection conn = createConnection();
        assertEquals(0, conn.getNetworkTimeout());
        conn.setNetworkTimeout(null, 5000);
    }

    @Test
    void testAbort() throws SQLException {
        PrimusDBConnection conn = createConnection();
        conn.abort(null);
        assertTrue(conn.isClosed());
    }

    @Test
    void testUnwrap() throws SQLException {
        PrimusDBConnection conn = createConnection();
        assertFalse(conn.isWrapperFor(PrimusDBConnection.class));
        assertThrows(SQLFeatureNotSupportedException.class, () -> conn.unwrap(PrimusDBConnection.class));
    }

    @Test
    void testCreateStatementOverloads() throws SQLException {
        PrimusDBConnection conn = createConnection();
        assertNotNull(conn.createStatement(ResultSet.TYPE_FORWARD_ONLY, ResultSet.CONCUR_READ_ONLY));
        assertNotNull(conn.createStatement(ResultSet.TYPE_FORWARD_ONLY, ResultSet.CONCUR_READ_ONLY, ResultSet.CLOSE_CURSORS_AT_COMMIT));
    }

    @Test
    void testPrepareCallThrows() throws SQLException {
        PrimusDBConnection conn = createConnection();
        assertThrows(SQLFeatureNotSupportedException.class, () -> conn.prepareCall("SELECT 1"));
        assertThrows(SQLFeatureNotSupportedException.class, () -> conn.prepareCall("SELECT 1", ResultSet.TYPE_FORWARD_ONLY, ResultSet.CONCUR_READ_ONLY));
        assertThrows(SQLFeatureNotSupportedException.class, () -> conn.prepareCall("SELECT 1", ResultSet.TYPE_FORWARD_ONLY, ResultSet.CONCUR_READ_ONLY, ResultSet.CLOSE_CURSORS_AT_COMMIT));
    }

    @Test
    void testPrepareStatementOverloads() throws SQLException {
        PrimusDBConnection conn = createConnection();
        assertNotNull(conn.prepareStatement("SELECT 1"));
        assertNotNull(conn.prepareStatement("SELECT 1", ResultSet.TYPE_FORWARD_ONLY, ResultSet.CONCUR_READ_ONLY));
        assertNotNull(conn.prepareStatement("SELECT 1", ResultSet.TYPE_FORWARD_ONLY, ResultSet.CONCUR_READ_ONLY, ResultSet.CLOSE_CURSORS_AT_COMMIT));
        assertNotNull(conn.prepareStatement("SELECT 1", Statement.RETURN_GENERATED_KEYS));
        assertNotNull(conn.prepareStatement("SELECT 1", new int[]{1}));
        assertNotNull(conn.prepareStatement("SELECT 1", new String[]{"id"}));
    }

    @Test
    void testHttpClient() {
        PrimusDBConnection conn = createConnection();
        assertNotNull(conn.getHttpClient());
    }
}
