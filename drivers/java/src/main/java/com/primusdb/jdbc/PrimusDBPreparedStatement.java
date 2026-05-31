package com.primusdb.jdbc;

import java.sql.*;
import java.io.InputStream;
import java.io.Reader;
import java.math.BigDecimal;
import java.net.URL;
import java.util.*;
import java.sql.Date;

public class PrimusDBPreparedStatement extends PrimusDBStatement implements PreparedStatement {

    private final String sqlTemplate;
    private final List<Object> parameters;
    private final List<int[]> batchParameters;

    public PrimusDBPreparedStatement(PrimusDBConnection connection, String sql) {
        super(connection);
        this.sqlTemplate = sql;
        this.parameters = new ArrayList<>();
        this.batchParameters = new ArrayList<>();
    }

    private void setParam(int parameterIndex, Object value) throws SQLException {
        if (parameterIndex < 1) {
            throw new SQLException("Parameter index must be >= 1");
        }
        while (parameters.size() < parameterIndex) {
            parameters.add(null);
        }
        parameters.set(parameterIndex - 1, value);
    }

    private String buildSql() throws SQLException {
        StringBuilder result = new StringBuilder();
        int paramIdx = 0;
        int lastIdx = 0;
        String sql = sqlTemplate;
        while (true) {
            int qm = sql.indexOf('?', lastIdx);
            if (qm == -1) {
                result.append(sql.substring(lastIdx));
                break;
            }
            result.append(sql, lastIdx, qm);
            if (paramIdx >= parameters.size()) {
                throw new SQLException("Too few parameters for SQL: " + sqlTemplate);
            }
            Object val = parameters.get(paramIdx);
            result.append(escapeValue(val));
            paramIdx++;
            lastIdx = qm + 1;
        }
        return result.toString();
    }

    private String escapeValue(Object val) {
        if (val == null) {
            return "NULL";
        }
        if (val instanceof Number || val instanceof Boolean) {
            return val.toString();
        }
        String str = val.toString();
        return "'" + str.replace("\\", "\\\\").replace("'", "\\'") + "'";
    }

    @Override
    public ResultSet executeQuery() throws SQLException {
        return executeQuery(buildSql());
    }

    @Override
    public int executeUpdate() throws SQLException {
        return executeUpdate(buildSql());
    }

    @Override
    public boolean execute() throws SQLException {
        return execute(buildSql());
    }

    @Override
    public void setNull(int parameterIndex, int sqlType) throws SQLException {
        setParam(parameterIndex, null);
    }

    @Override
    public void setNull(int parameterIndex, int sqlType, String typeName) throws SQLException {
        setParam(parameterIndex, null);
    }

    @Override
    public void setBoolean(int parameterIndex, boolean x) throws SQLException {
        setParam(parameterIndex, x);
    }

    @Override
    public void setByte(int parameterIndex, byte x) throws SQLException {
        setParam(parameterIndex, x);
    }

    @Override
    public void setShort(int parameterIndex, short x) throws SQLException {
        setParam(parameterIndex, x);
    }

    @Override
    public void setInt(int parameterIndex, int x) throws SQLException {
        setParam(parameterIndex, x);
    }

    @Override
    public void setLong(int parameterIndex, long x) throws SQLException {
        setParam(parameterIndex, x);
    }

    @Override
    public void setFloat(int parameterIndex, float x) throws SQLException {
        setParam(parameterIndex, x);
    }

    @Override
    public void setDouble(int parameterIndex, double x) throws SQLException {
        setParam(parameterIndex, x);
    }

    @Override
    public void setBigDecimal(int parameterIndex, BigDecimal x) throws SQLException {
        setParam(parameterIndex, x);
    }

    @Override
    public void setString(int parameterIndex, String x) throws SQLException {
        setParam(parameterIndex, x);
    }

    @Override
    public void setBytes(int parameterIndex, byte[] x) throws SQLException {
        setParam(parameterIndex, x != null ? Base64.getEncoder().encodeToString(x) : null);
    }

    @Override
    public void setDate(int parameterIndex, Date x) throws SQLException {
        setParam(parameterIndex, x != null ? x.toString() : null);
    }

    @Override
    public void setDate(int parameterIndex, Date x, Calendar cal) throws SQLException {
        setParam(parameterIndex, x != null ? x.toString() : null);
    }

    @Override
    public void setTime(int parameterIndex, Time x) throws SQLException {
        setParam(parameterIndex, x != null ? x.toString() : null);
    }

    @Override
    public void setTime(int parameterIndex, Time x, Calendar cal) throws SQLException {
        setParam(parameterIndex, x != null ? x.toString() : null);
    }

    @Override
    public void setTimestamp(int parameterIndex, Timestamp x) throws SQLException {
        setParam(parameterIndex, x != null ? x.toString() : null);
    }

    @Override
    public void setTimestamp(int parameterIndex, Timestamp x, Calendar cal) throws SQLException {
        setParam(parameterIndex, x != null ? x.toString() : null);
    }

    @Override
    public void setObject(int parameterIndex, Object x) throws SQLException {
        setParam(parameterIndex, x);
    }

    @Override
    public void setObject(int parameterIndex, Object x, int targetSqlType) throws SQLException {
        setParam(parameterIndex, x);
    }

    @Override
    public void setObject(int parameterIndex, Object x, int targetSqlType, int scaleOrLength) throws SQLException {
        setParam(parameterIndex, x);
    }

    @Override
    public void clearParameters() throws SQLException {
        parameters.clear();
    }

    @Override
    public void addBatch() throws SQLException {
        int[] snapshot = new int[parameters.size()];
        for (int i = 0; i < parameters.size(); i++) {
            Object val = parameters.get(i);
            snapshot[i] = val != null ? val.hashCode() : 0;
        }
        batchParameters.add(snapshot);
    }

    @Override
    public void clearBatch() throws SQLException {
        batchParameters.clear();
    }

    @Override
    public int[] executeBatch() throws SQLException {
        int[] results = new int[batchParameters.size()];
        for (int i = 0; i < batchParameters.size(); i++) {
            int[] snapshot = batchParameters.get(i);
            for (int j = 0; j < snapshot.length && j < parameters.size(); j++) {
                Object original = parameters.get(j);
                if (original instanceof Number) {
                    setParam(j + 1, snapshot[j]);
                }
            }
            results[i] = executeUpdate();
        }
        batchParameters.clear();
        return results;
    }

    @Override
    public void setAsciiStream(int parameterIndex, InputStream x, int length) throws SQLException {
        throw new SQLFeatureNotSupportedException("AsciiStream not supported");
    }

    @Override
    public void setAsciiStream(int parameterIndex, InputStream x, long length) throws SQLException {
        throw new SQLFeatureNotSupportedException("AsciiStream not supported");
    }

    @Override
    public void setAsciiStream(int parameterIndex, InputStream x) throws SQLException {
        throw new SQLFeatureNotSupportedException("AsciiStream not supported");
    }

    @Override
    public void setUnicodeStream(int parameterIndex, InputStream x, int length) throws SQLException {
        throw new SQLFeatureNotSupportedException("UnicodeStream not supported");
    }

    @Override
    public void setBinaryStream(int parameterIndex, InputStream x, int length) throws SQLException {
        throw new SQLFeatureNotSupportedException("BinaryStream not supported");
    }

    @Override
    public void setBinaryStream(int parameterIndex, InputStream x, long length) throws SQLException {
        throw new SQLFeatureNotSupportedException("BinaryStream not supported");
    }

    @Override
    public void setBinaryStream(int parameterIndex, InputStream x) throws SQLException {
        throw new SQLFeatureNotSupportedException("BinaryStream not supported");
    }

    @Override
    public void setCharacterStream(int parameterIndex, Reader reader, int length) throws SQLException {
        throw new SQLFeatureNotSupportedException("CharacterStream not supported");
    }

    @Override
    public void setCharacterStream(int parameterIndex, Reader reader, long length) throws SQLException {
        throw new SQLFeatureNotSupportedException("CharacterStream not supported");
    }

    @Override
    public void setCharacterStream(int parameterIndex, Reader reader) throws SQLException {
        throw new SQLFeatureNotSupportedException("CharacterStream not supported");
    }

    @Override
    public void setNCharacterStream(int parameterIndex, Reader value, long length) throws SQLException {
        throw new SQLFeatureNotSupportedException("NCharacterStream not supported");
    }

    @Override
    public void setNCharacterStream(int parameterIndex, Reader value) throws SQLException {
        throw new SQLFeatureNotSupportedException("NCharacterStream not supported");
    }

    @Override
    public void setRef(int parameterIndex, Ref x) throws SQLException {
        throw new SQLFeatureNotSupportedException("Ref not supported");
    }

    @Override
    public void setBlob(int parameterIndex, Blob x) throws SQLException {
        throw new SQLFeatureNotSupportedException("Blob not supported");
    }

    @Override
    public void setBlob(int parameterIndex, InputStream inputStream) throws SQLException {
        throw new SQLFeatureNotSupportedException("Blob not supported");
    }

    @Override
    public void setBlob(int parameterIndex, InputStream inputStream, long length) throws SQLException {
        throw new SQLFeatureNotSupportedException("Blob not supported");
    }

    @Override
    public void setClob(int parameterIndex, Clob x) throws SQLException {
        throw new SQLFeatureNotSupportedException("Clob not supported");
    }

    @Override
    public void setClob(int parameterIndex, Reader reader) throws SQLException {
        throw new SQLFeatureNotSupportedException("Clob not supported");
    }

    @Override
    public void setClob(int parameterIndex, Reader reader, long length) throws SQLException {
        throw new SQLFeatureNotSupportedException("Clob not supported");
    }

    @Override
    public void setArray(int parameterIndex, Array x) throws SQLException {
        throw new SQLFeatureNotSupportedException("Array not supported");
    }

    @Override
    public void setURL(int parameterIndex, URL x) throws SQLException {
        setParam(parameterIndex, x != null ? x.toString() : null);
    }

    @Override
    public void setNString(int parameterIndex, String value) throws SQLException {
        setString(parameterIndex, value);
    }

    @Override
    public void setNClob(int parameterIndex, NClob value) throws SQLException {
        throw new SQLFeatureNotSupportedException("NClob not supported");
    }

    @Override
    public void setNClob(int parameterIndex, Reader reader) throws SQLException {
        throw new SQLFeatureNotSupportedException("NClob not supported");
    }

    @Override
    public void setNClob(int parameterIndex, Reader reader, long length) throws SQLException {
        throw new SQLFeatureNotSupportedException("NClob not supported");
    }

    @Override
    public void setSQLXML(int parameterIndex, SQLXML xmlObject) throws SQLException {
        setParam(parameterIndex, xmlObject != null ? xmlObject.getString() : null);
    }

    @Override
    public void setRowId(int parameterIndex, RowId x) throws SQLException {
        setParam(parameterIndex, x != null ? x.toString() : null);
    }

    @Override
    public ParameterMetaData getParameterMetaData() throws SQLException {
        return null;
    }

    @Override
    public ResultSetMetaData getMetaData() throws SQLException {
        return null;
    }
}
