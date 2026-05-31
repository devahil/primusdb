package com.primusdb.jdbc;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;
import java.sql.*;

class PrimusDBResultSetTest {

    private PrimusDBResultSet resultSet;

    @BeforeEach
    void setUp() throws SQLException {
        JsonArray data = new JsonArray();

        JsonObject row1 = new JsonObject();
        row1.addProperty("id", 1);
        row1.addProperty("name", "Alice");
        row1.addProperty("score", 95.5);
        row1.addProperty("active", true);
        row1.add("nullable", com.google.gson.JsonNull.INSTANCE);
        data.add(row1);

        JsonObject row2 = new JsonObject();
        row2.addProperty("id", 2);
        row2.addProperty("name", "Bob");
        row2.addProperty("score", 87.0);
        row2.addProperty("active", false);
        data.add(row2);

        resultSet = new PrimusDBResultSet(data);
    }

    @Test
    void testNext() throws SQLException {
        assertTrue(resultSet.next());
        assertTrue(resultSet.next());
        assertFalse(resultSet.next());
    }

    @Test
    void testGetString() throws SQLException {
        resultSet.next();
        assertEquals("Alice", resultSet.getString("name"));
    }

    @Test
    void testGetInt() throws SQLException {
        resultSet.next();
        assertEquals(1, resultSet.getInt("id"));
    }

    @Test
    void testGetDouble() throws SQLException {
        resultSet.next();
        assertEquals(95.5, resultSet.getDouble("score"), 0.001);
    }

    @Test
    void testGetBoolean() throws SQLException {
        resultSet.next();
        assertTrue(resultSet.getBoolean("active"));
        resultSet.next();
        assertFalse(resultSet.getBoolean("active"));
    }

    @Test
    void testGetObject() throws SQLException {
        resultSet.next();
        assertNotNull(resultSet.getObject("name"));
    }

    @Test
    void testWasNull() throws SQLException {
        resultSet.next();
        assertEquals("Alice", resultSet.getString("name"));
    }

    @Test
    void testGetNonExistentColumn() throws SQLException {
        resultSet.next();
        assertNull(resultSet.getString("nonexistent"));
    }

    @Test
    void testGetNullColumn() throws SQLException {
        resultSet.next();
        assertNull(resultSet.getString("nullable"));
    }

    @Test
    void testClose() throws SQLException {
        resultSet.close();
        assertTrue(resultSet.isClosed());
    }

    @Test
    void testOperationsOnClosedResultSet() throws SQLException {
        resultSet.close();
        assertThrows(SQLException.class, () -> resultSet.next());
        assertThrows(SQLException.class, () -> resultSet.getString("name"));
        assertThrows(SQLException.class, () -> resultSet.getInt("id"));
    }

    @Test
    void testFindColumn() throws SQLException {
        assertEquals(1, resultSet.findColumn("any"));
    }

    @Test
    void testGetStatement() throws SQLException {
        assertNull(resultSet.getStatement());
    }

    @Test
    void testGetMetaData() throws SQLException {
        assertNull(resultSet.getMetaData());
    }

    @Test
    void testGetWarnings() throws SQLException {
        assertNull(resultSet.getWarnings());
    }

    @Test
    void testClearWarnings() throws SQLException {
        resultSet.clearWarnings();
    }

    @Test
    void testGetCursorName() throws SQLException {
        assertNull(resultSet.getCursorName());
    }

    @Test
    void testGetType() throws SQLException {
        assertEquals(ResultSet.TYPE_FORWARD_ONLY, resultSet.getType());
    }

    @Test
    void testGetConcurrency() throws SQLException {
        assertEquals(ResultSet.CONCUR_READ_ONLY, resultSet.getConcurrency());
    }

    @Test
    void testGetHoldability() throws SQLException {
        assertEquals(ResultSet.CLOSE_CURSORS_AT_COMMIT, resultSet.getHoldability());
    }

    @Test
    void testRowNavigation() throws SQLException {
        assertFalse(resultSet.absolute(1));
        assertFalse(resultSet.relative(1));
        assertFalse(resultSet.previous());
        assertEquals(0, resultSet.getRow());
        assertFalse(resultSet.first());
        assertFalse(resultSet.last());
        assertFalse(resultSet.isBeforeFirst());
        assertFalse(resultSet.isAfterLast());
        assertFalse(resultSet.isFirst());
        assertFalse(resultSet.isLast());
        resultSet.beforeFirst();
        resultSet.afterLast();
    }

    @Test
    void testFetchDirection() throws SQLException {
        assertEquals(ResultSet.FETCH_FORWARD, resultSet.getFetchDirection());
        resultSet.setFetchDirection(ResultSet.FETCH_FORWARD);
    }

    @Test
    void testFetchSize() throws SQLException {
        assertEquals(0, resultSet.getFetchSize());
        resultSet.setFetchSize(10);
    }

    @Test
    void testRowUpdatedInsertedDeleted() throws SQLException {
        assertFalse(resultSet.rowUpdated());
        assertFalse(resultSet.rowInserted());
        assertFalse(resultSet.rowDeleted());
    }

    @Test
    void testUpdateMethodsThrow() throws SQLException {
        assertThrows(SQLFeatureNotSupportedException.class, () -> resultSet.updateNull(1));
        assertThrows(SQLFeatureNotSupportedException.class, () -> resultSet.updateString("name", "test"));
        assertThrows(SQLFeatureNotSupportedException.class, () -> resultSet.updateRow());
        assertThrows(SQLFeatureNotSupportedException.class, () -> resultSet.insertRow());
        assertThrows(SQLFeatureNotSupportedException.class, () -> resultSet.deleteRow());
    }

    @Test
    void testGetBigDecimal() throws SQLException {
        assertNull(resultSet.getBigDecimal(1));
        assertNull(resultSet.getBigDecimal("col"));
        assertNull(resultSet.getBigDecimal(1, 2));
        assertNull(resultSet.getBigDecimal("col", 2));
    }

    @Test
    void testGetBytes() throws SQLException {
        assertNull(resultSet.getBytes(1));
        assertNull(resultSet.getBytes("col"));
    }

    @Test
    void testGetDate() throws SQLException {
        assertNull(resultSet.getDate(1));
        assertNull(resultSet.getDate("col"));
        assertNull(resultSet.getDate(1, java.util.Calendar.getInstance()));
        assertNull(resultSet.getDate("col", java.util.Calendar.getInstance()));
    }

    @Test
    void testGetTime() throws SQLException {
        assertNull(resultSet.getTime(1));
        assertNull(resultSet.getTime("col"));
        assertNull(resultSet.getTime(1, java.util.Calendar.getInstance()));
        assertNull(resultSet.getTime("col", java.util.Calendar.getInstance()));
    }

    @Test
    void testGetTimestamp() throws SQLException {
        assertNull(resultSet.getTimestamp(1));
        assertNull(resultSet.getTimestamp("col"));
        assertNull(resultSet.getTimestamp(1, java.util.Calendar.getInstance()));
        assertNull(resultSet.getTimestamp("col", java.util.Calendar.getInstance()));
    }

    @Test
    void testGetBinaryStream() throws SQLException {
        assertNull(resultSet.getBinaryStream(1));
        assertNull(resultSet.getBinaryStream("col"));
    }

    @Test
    void testGetAsciiStream() throws SQLException {
        assertNull(resultSet.getAsciiStream(1));
        assertNull(resultSet.getAsciiStream("col"));
    }

    @Test
    void testGetUnicodeStream() throws SQLException {
        assertNull(resultSet.getUnicodeStream(1));
        assertNull(resultSet.getUnicodeStream("col"));
    }

    @Test
    void testGetCharacterStream() throws SQLException {
        assertNull(resultSet.getCharacterStream(1));
        assertNull(resultSet.getCharacterStream("col"));
    }

    @Test
    void testGetObjectWithType() throws SQLException {
        assertNull(resultSet.getObject(1, String.class));
        assertNull(resultSet.getObject("col", String.class));
    }

    @Test
    void testGetObjectWithMap() throws SQLException {
        assertNull(resultSet.getObject(1, new java.util.HashMap<>()));
        assertNull(resultSet.getObject("col", new java.util.HashMap<>()));
    }

    @Test
    void testGetURL() throws SQLException {
        assertNull(resultSet.getURL(1));
        assertNull(resultSet.getURL("col"));
    }

    @Test
    void testGetRef() throws SQLException {
        assertNull(resultSet.getRef(1));
        assertNull(resultSet.getRef("col"));
    }

    @Test
    void testGetBlob() throws SQLException {
        assertNull(resultSet.getBlob(1));
        assertNull(resultSet.getBlob("col"));
    }

    @Test
    void testGetClob() throws SQLException {
        assertNull(resultSet.getClob(1));
        assertNull(resultSet.getClob("col"));
    }

    @Test
    void testGetArray() throws SQLException {
        assertNull(resultSet.getArray(1));
        assertNull(resultSet.getArray("col"));
    }

    @Test
    void testGetRowId() throws SQLException {
        assertNull(resultSet.getRowId(1));
        assertNull(resultSet.getRowId("col"));
    }

    @Test
    void testGetNString() throws SQLException {
        assertNull(resultSet.getNString(1));
        assertNull(resultSet.getNString("col"));
    }

    @Test
    void testGetNCharacterStream() throws SQLException {
        assertNull(resultSet.getNCharacterStream(1));
        assertNull(resultSet.getNCharacterStream("col"));
    }

    @Test
    void testGetNClob() throws SQLException {
        assertNull(resultSet.getNClob(1));
        assertNull(resultSet.getNClob("col"));
    }

    @Test
    void testGetSQLXML() throws SQLException {
        assertNull(resultSet.getSQLXML(1));
        assertNull(resultSet.getSQLXML("col"));
    }

    @Test
    void testUpdateRowIdThrows() throws SQLException {
        assertThrows(SQLFeatureNotSupportedException.class, () -> resultSet.updateRowId(1, null));
        assertThrows(SQLFeatureNotSupportedException.class, () -> resultSet.updateRowId("col", null));
    }

    @Test
    void testUnwrap() throws SQLException {
        assertFalse(resultSet.isWrapperFor(PrimusDBResultSet.class));
        assertThrows(SQLFeatureNotSupportedException.class, () -> resultSet.unwrap(PrimusDBResultSet.class));
    }
}
