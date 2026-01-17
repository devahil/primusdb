package com.primusdb.jdbc;

import java.sql.*;
import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import java.util.Iterator;

/**
 * JDBC ResultSet implementation for PrimusDB
 */
public class PrimusDBResultSet implements ResultSet {

    private final JsonArray results;
    private final Iterator<JsonElement> iterator;
    private JsonObject currentRow;
    private boolean closed = false;
    private int rowIndex = 0;

    public PrimusDBResultSet(JsonArray results) {
        this.results = results;
        this.iterator = results.iterator();
        if (iterator.hasNext()) {
            this.currentRow = iterator.next().getAsJsonObject();
        }
    }

    @Override
    public boolean next() throws SQLException {
        checkClosed();
        if (iterator.hasNext()) {
            currentRow = iterator.next().getAsJsonObject();
            rowIndex++;
            return true;
        }
        return false;
    }

    @Override
    public String getString(String columnLabel) throws SQLException {
        checkClosed();
        if (currentRow != null && currentRow.has(columnLabel)) {
            JsonElement element = currentRow.get(columnLabel);
            return element.isJsonNull() ? null : element.getAsString();
        }
        return null;
    }

    @Override
    public int getInt(String columnLabel) throws SQLException {
        checkClosed();
        if (currentRow != null && currentRow.has(columnLabel)) {
            JsonElement element = currentRow.get(columnLabel);
            return element.isJsonNull() ? 0 : element.getAsInt();
        }
        return 0;
    }

    @Override
    public double getDouble(String columnLabel) throws SQLException {
        checkClosed();
        if (currentRow != null && currentRow.has(columnLabel)) {
            JsonElement element = currentRow.get(columnLabel);
            return element.isJsonNull() ? 0.0 : element.getAsDouble();
        }
        return 0.0;
    }

    @Override
    public boolean getBoolean(String columnLabel) throws SQLException {
        checkClosed();
        if (currentRow != null && currentRow.has(columnLabel)) {
            JsonElement element = currentRow.get(columnLabel);
            return element.isJsonNull() ? false : element.getAsBoolean();
        }
        return false;
    }

    @Override
    public Object getObject(String columnLabel) throws SQLException {
        checkClosed();
        if (currentRow != null && currentRow.has(columnLabel)) {
            JsonElement element = currentRow.get(columnLabel);
            return element.isJsonNull() ? null : element;
        }
        return null;
    }

    @Override
    public void close() throws SQLException {
        closed = true;
    }

    @Override
    public boolean isClosed() throws SQLException {
        return closed;
    }

    // Minimal implementations
    private void checkClosed() throws SQLException {
        if (closed) {
            throw new SQLException("ResultSet is closed");
        }
    }

    // Stub implementations for other methods
    @Override public boolean wasNull() throws SQLException { return false; }
    @Override public String getString(int columnIndex) throws SQLException { return getString("col" + columnIndex); }
    @Override public boolean getBoolean(int columnIndex) throws SQLException { return getBoolean("col" + columnIndex); }
    @Override public byte getByte(int columnIndex) throws SQLException { return 0; }
    @Override public short getShort(int columnIndex) throws SQLException { return 0; }
    @Override public int getInt(int columnIndex) throws SQLException { return getInt("col" + columnIndex); }
    @Override public long getLong(int columnIndex) throws SQLException { return 0; }
    @Override public float getFloat(int columnIndex) throws SQLException { return 0; }
    @Override public double getDouble(int columnIndex) throws SQLException { return getDouble("col" + columnIndex); }
    @Override public byte[] getBytes(int columnIndex) throws SQLException { return null; }
    @Override public java.sql.Date getDate(int columnIndex) throws SQLException { return null; }
    @Override public java.sql.Time getTime(int columnIndex) throws SQLException { return null; }
    @Override public java.sql.Timestamp getTimestamp(int columnIndex) throws SQLException { return null; }
    @Override public java.io.InputStream getAsciiStream(int columnIndex) throws SQLException { return null; }
    @Override public java.io.InputStream getUnicodeStream(int columnIndex) throws SQLException { return null; }
    @Override public java.io.InputStream getBinaryStream(int columnIndex) throws SQLException { return null; }
    @Override public String getString(int columnIndex) throws SQLException { return getString("col" + columnIndex); }
    @Override public boolean getBoolean(int columnIndex) throws SQLException { return getBoolean("col" + columnIndex); }
    @Override public byte getByte(String columnLabel) throws SQLException { return 0; }
    @Override public short getShort(String columnLabel) throws SQLException { return 0; }
    @Override public int getInt(String columnLabel) throws SQLException { return getInt(columnLabel); }
    @Override public long getLong(String columnLabel) throws SQLException { return 0; }
    @Override public float getFloat(String columnLabel) throws SQLException { return 0; }
    @Override public double getDouble(String columnLabel) throws SQLException { return getDouble(columnLabel); }
    @Override public byte[] getBytes(String columnLabel) throws SQLException { return null; }
    @Override public java.sql.Date getDate(String columnLabel) throws SQLException { return null; }
    @Override public java.sql.Time getTime(String columnLabel) throws SQLException { return null; }
    @Override public java.sql.Timestamp getTimestamp(String columnLabel) throws SQLException { return null; }
    @Override public java.io.InputStream getAsciiStream(String columnLabel) throws SQLException { return null; }
    @Override public java.io.InputStream getUnicodeStream(String columnLabel) throws SQLException { return null; }
    @Override public java.io.InputStream getBinaryStream(String columnLabel) throws SQLException { return null; }
    @Override public Object getObject(int columnIndex) throws SQLException { return getObject("col" + columnIndex); }
    @Override public int findColumn(String columnLabel) throws SQLException { return 1; }
    @Override public java.io.Reader getCharacterStream(int columnIndex) throws SQLException { return null; }
    @Override public java.io.Reader getCharacterStream(String columnLabel) throws SQLException { return null; }
    @Override public java.math.BigDecimal getBigDecimal(int columnIndex) throws SQLException { return null; }
    @Override public java.math.BigDecimal getBigDecimal(String columnLabel) throws SQLException { return null; }
    @Override public SQLWarning getWarnings() throws SQLException { return null; }
    @Override public void clearWarnings() throws SQLException {}
    @Override public String getCursorName() throws SQLException { return null; }
    @Override public ResultSetMetaData getMetaData() throws SQLException { return null; }
    @Override public Object getObject(int columnIndex, java.util.Map<String,Class<?>> map) throws SQLException { return null; }
    @Override public Object getObject(String columnLabel, java.util.Map<String,Class<?>> map) throws SQLException { return null; }
    @Override public <T> T getObject(int columnIndex, Class<T> type) throws SQLException { return null; }
    @Override public <T> T getObject(String columnLabel, Class<T> type) throws SQLException { return null; }
    @Override public java.net.URL getURL(int columnIndex) throws SQLException { return null; }
    @Override public java.net.URL getURL(String columnLabel) throws SQLException { return null; }
    @Override public void updateNull(int columnIndex) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateBoolean(int columnIndex, boolean x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateByte(int columnIndex, byte x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateShort(int columnIndex, short x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateInt(int columnIndex, int x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateLong(int columnIndex, long x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateFloat(int columnIndex, float x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateDouble(int columnIndex, double x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateBigDecimal(int columnIndex, java.math.BigDecimal x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateString(int columnIndex, String x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateBytes(int columnIndex, byte[] x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateDate(int columnIndex, java.sql.Date x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateTime(int columnIndex, java.sql.Time x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateTimestamp(int columnIndex, java.sql.Timestamp x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateAsciiStream(int columnIndex, java.io.InputStream x, int length) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateBinaryStream(int columnIndex, java.io.InputStream x, int length) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateCharacterStream(int columnIndex, java.io.Reader x, int length) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateObject(int columnIndex, Object x, int scaleOrLength) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateObject(int columnIndex, Object x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateNull(String columnLabel) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateBoolean(String columnLabel, boolean x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateByte(String columnLabel, byte x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateShort(String columnLabel, short x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateInt(String columnLabel, int x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateLong(String columnLabel, long x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateFloat(String columnLabel, float x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateDouble(String columnLabel, double x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateBigDecimal(String columnLabel, java.math.BigDecimal x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateString(String columnLabel, String x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateBytes(String columnLabel, byte[] x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateDate(String columnLabel, java.sql.Date x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateTime(String columnLabel, java.sql.Time x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateTimestamp(String columnLabel, java.sql.Timestamp x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateAsciiStream(String columnLabel, java.io.InputStream x, int length) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateBinaryStream(String columnLabel, java.io.InputStream x, int length) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateCharacterStream(String columnLabel, java.io.Reader x, int length) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateObject(String columnLabel, Object x, int scaleOrLength) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateObject(String columnLabel, Object x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void insertRow() throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateRow() throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void deleteRow() throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void refreshRow() throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void cancelRowUpdates() throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void moveToInsertRow() throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void moveToCurrentRow() throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public Statement getStatement() throws SQLException { return null; }
    @Override public java.io.InputStream getUnicodeStream(int columnIndex) throws SQLException { return null; }
    @Override public java.io.InputStream getUnicodeStream(String columnLabel) throws SQLException { return null; }
    @Override public java.io.InputStream getBinaryStream(int columnIndex) throws SQLException { return null; }
    @Override public java.io.InputStream getBinaryStream(String columnLabel) throws SQLException { return null; }
    @Override public boolean absolute(int row) throws SQLException { return false; }
    @Override public boolean relative(int rows) throws SQLException { return false; }
    @Override public boolean previous() throws SQLException { return false; }
    @Override public void setFetchDirection(int direction) throws SQLException {}
    @Override public int getFetchDirection() throws SQLException { return FETCH_FORWARD; }
    @Override public void setFetchSize(int rows) throws SQLException {}
    @Override public int getFetchSize() throws SQLException { return 0; }
    @Override public int getType() throws SQLException { return TYPE_FORWARD_ONLY; }
    @Override public int getConcurrency() throws SQLException { return CONCUR_READ_ONLY; }
    @Override public boolean rowUpdated() throws SQLException { return false; }
    @Override public boolean rowInserted() throws SQLException { return false; }
    @Override public boolean rowDeleted() throws SQLException { return false; }
    @Override public void updateRef(int columnIndex, java.sql.Ref x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateRef(String columnLabel, java.sql.Ref x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateBlob(int columnIndex, java.sql.Blob x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateBlob(String columnLabel, java.sql.Blob x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateClob(int columnIndex, java.sql.Clob x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateClob(String columnLabel, java.sql.Clob x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateArray(int columnIndex, java.sql.Array x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateArray(String columnLabel, java.sql.Array x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public java.sql.Ref getRef(int columnIndex) throws SQLException { return null; }
    @Override public java.sql.Ref getRef(String columnLabel) throws SQLException { return null; }
    @Override public java.sql.Blob getBlob(int columnIndex) throws SQLException { return null; }
    @Override public java.sql.Blob getBlob(String columnLabel) throws SQLException { return null; }
    @Override public java.sql.Clob getClob(int columnIndex) throws SQLException { return null; }
    @Override public java.sql.Clob getClob(String columnLabel) throws SQLException { return null; }
    @Override public java.sql.Array getArray(int columnIndex) throws SQLException { return null; }
    @Override public java.sql.Array getArray(String columnLabel) throws SQLException { return null; }
    @Override public java.sql.Date getDate(int columnIndex, java.util.Calendar cal) throws SQLException { return null; }
    @Override public java.sql.Date getDate(String columnLabel, java.util.Calendar cal) throws SQLException { return null; }
    @Override public java.sql.Time getTime(int columnIndex, java.util.Calendar cal) throws SQLException { return null; }
    @Override public java.sql.Time getTime(String columnLabel, java.util.Calendar cal) throws SQLException { return null; }
    @Override public java.sql.Timestamp getTimestamp(int columnIndex, java.util.Calendar cal) throws SQLException { return null; }
    @Override public java.sql.Timestamp getTimestamp(String columnLabel, java.util.Calendar cal) throws SQLException { return null; }
    @Override public java.sql.RowId getRowId(int columnIndex) throws SQLException { return null; }
    @Override public java.sql.RowId getRowId(String columnLabel) throws SQLException { return null; }
    @Override public void updateRowId(int columnIndex, java.sql.RowId x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public void updateRowId(String columnLabel, java.sql.RowId x) throws SQLException { throw new SQLFeatureNotSupportedException(); }
    @Override public int getHoldability() throws SQLException { return CLOSE_CURSORS_AT_COMMIT; }
    @Override public boolean isWrapperFor(Class<?> iface) throws SQLException { return false; }
    @Override public <T> T unwrap(Class<T> iface) throws SQLException { throw new SQLFeatureNotSupportedException(); }
}