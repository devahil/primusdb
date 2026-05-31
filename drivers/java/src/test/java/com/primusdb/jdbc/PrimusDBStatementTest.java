package com.primusdb.jdbc;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;
import java.sql.*;

class PrimusDBStatementTest {

    private PrimusDBStatement statement;

    @BeforeEach
    void setUp() throws SQLException {
        PrimusDBConnection conn = new PrimusDBConnection("localhost", 8080, "testdb", "user", "pass");
        statement = (PrimusDBStatement) conn.createStatement();
    }

    @Test
    void testClose() throws SQLException {
        assertFalse(statement.isClosed());
        statement.close();
        assertTrue(statement.isClosed());
    }

    @Test
    void testOperationsOnClosedStatement() throws SQLException {
        statement.close();
        assertThrows(SQLException.class, () -> statement.executeQuery("SELECT 1"));
        assertThrows(SQLException.class, () -> statement.executeUpdate("INSERT INTO test VALUES (1)"));
        assertThrows(SQLException.class, () -> statement.execute("SELECT 1"));
    }

    @Test
    void testGetConnection() throws SQLException {
        assertNotNull(statement.getConnection());
        assertTrue(statement.getConnection() instanceof PrimusDBConnection);
    }

    @Test
    void testGetUpdateCount() throws SQLException {
        assertEquals(-1, statement.getUpdateCount());
    }

    @Test
    void testGetMoreResults() throws SQLException {
        assertFalse(statement.getMoreResults());
        assertFalse(statement.getMoreResults(Statement.CLOSE_ALL_RESULTS));
    }

    @Test
    void testGetResultSet() throws SQLException {
        assertNull(statement.getResultSet());
    }

    @Test
    void testGetGeneratedKeys() throws SQLException {
        assertNull(statement.getGeneratedKeys());
    }

    @Test
    void testSetFetchDirection() throws SQLException {
        statement.setFetchDirection(ResultSet.FETCH_FORWARD);
        assertEquals(ResultSet.FETCH_FORWARD, statement.getFetchDirection());
    }

    @Test
    void testSetFetchSize() throws SQLException {
        statement.setFetchSize(100);
        assertEquals(0, statement.getFetchSize());
    }

    @Test
    void testMaxFieldSize() throws SQLException {
        statement.setMaxFieldSize(100);
        assertEquals(0, statement.getMaxFieldSize());
    }

    @Test
    void testMaxRows() throws SQLException {
        statement.setMaxRows(100);
        assertEquals(0, statement.getMaxRows());
    }

    @Test
    void testQueryTimeout() throws SQLException {
        statement.setQueryTimeout(30);
        assertEquals(0, statement.getQueryTimeout());
    }

    @Test
    void testCancel() throws SQLException {
        statement.cancel();
    }

    @Test
    void testWarnings() throws SQLException {
        assertNull(statement.getWarnings());
        statement.clearWarnings();
    }

    @Test
    void testSetCursorName() throws SQLException {
        statement.setCursorName("cursor1");
    }

    @Test
    void testSetEscapeProcessing() throws SQLException {
        statement.setEscapeProcessing(true);
    }

    @Test
    void testGetResultSetConcurrency() throws SQLException {
        assertEquals(ResultSet.CONCUR_READ_ONLY, statement.getResultSetConcurrency());
    }

    @Test
    void testGetResultSetType() throws SQLException {
        assertEquals(ResultSet.TYPE_FORWARD_ONLY, statement.getResultSetType());
    }

    @Test
    void testGetResultSetHoldability() throws SQLException {
        assertEquals(ResultSet.CLOSE_CURSORS_AT_COMMIT, statement.getResultSetHoldability());
    }

    @Test
    void testIsPoolable() throws SQLException {
        assertFalse(statement.isPoolable());
        statement.setPoolable(true);
        assertFalse(statement.isPoolable());
    }

    @Test
    void testAddBatch() {
        assertThrows(SQLFeatureNotSupportedException.class, () -> statement.addBatch("SELECT 1"));
    }

    @Test
    void testClearBatch() throws SQLException {
        statement.clearBatch();
    }

    @Test
    void testExecuteBatch() {
        assertThrows(SQLFeatureNotSupportedException.class, () -> statement.executeBatch());
    }

    @Test
    void testCloseOnCompletion() throws SQLException {
        statement.closeOnCompletion();
        assertFalse(statement.isCloseOnCompletion());
    }

    @Test
    void testUnwrap() throws SQLException {
        assertFalse(statement.isWrapperFor(PrimusDBStatement.class));
        assertThrows(SQLFeatureNotSupportedException.class, () -> statement.unwrap(PrimusDBStatement.class));
    }

    @Test
    void testExecuteOverloads() throws SQLException {
        // These will throw because they attempt actual HTTP calls
        assertThrows(SQLException.class, () -> statement.execute("SELECT 1"));
        assertThrows(SQLException.class, () -> statement.execute("SELECT 1", Statement.RETURN_GENERATED_KEYS));
        assertThrows(SQLException.class, () -> statement.execute("SELECT 1", new int[]{1}));
        assertThrows(SQLException.class, () -> statement.execute("SELECT 1", new String[]{"col"}));
    }

    @Test
    void testExecuteUpdateOverloads() throws SQLException {
        assertThrows(SQLException.class, () -> statement.executeUpdate("SELECT 1"));
        assertThrows(SQLException.class, () -> statement.executeUpdate("SELECT 1", Statement.RETURN_GENERATED_KEYS));
        assertThrows(SQLException.class, () -> statement.executeUpdate("SELECT 1", new int[]{1}));
        assertThrows(SQLException.class, () -> statement.executeUpdate("SELECT 1", new String[]{"col"}));
    }

    @Test
    void testExecuteQueryFailsWithNoServer() {
        assertThrows(SQLException.class, () -> statement.executeQuery("SELECT * FROM test"));
    }
}
