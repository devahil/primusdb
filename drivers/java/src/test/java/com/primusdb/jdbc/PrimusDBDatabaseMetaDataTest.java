package com.primusdb.jdbc;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;
import java.sql.*;

class PrimusDBDatabaseMetaDataTest {

    private PrimusDBDatabaseMetaData meta;

    @BeforeEach
    void setUp() throws SQLException {
        PrimusDBConnection conn = new PrimusDBConnection("localhost", 8080, "testdb", "user", "pass");
        meta = (PrimusDBDatabaseMetaData) conn.getMetaData();
    }

    @Test
    void testDatabaseProductInfo() throws SQLException {
        assertEquals("PrimusDB", meta.getDatabaseProductName());
        assertEquals("0.1.0", meta.getDatabaseProductVersion());
    }

    @Test
    void testDriverInfo() throws SQLException {
        assertEquals("PrimusDB JDBC Driver", meta.getDriverName());
        assertEquals("0.1.0", meta.getDriverVersion());
        assertEquals(0, meta.getDriverMajorVersion());
        assertEquals(1, meta.getDriverMinorVersion());
    }

    @Test
    void testJDBCVersion() throws SQLException {
        assertEquals(4, meta.getJDBCMajorVersion());
        assertEquals(2, meta.getJDBCMinorVersion());
    }

    @Test
    void testDatabaseVersion() throws SQLException {
        assertEquals(0, meta.getDatabaseMajorVersion());
        assertEquals(1, meta.getDatabaseMinorVersion());
    }

    @Test
    void testURL() throws SQLException {
        assertEquals("jdbc:primusdb://localhost:8080", meta.getURL());
    }

    @Test
    void testUserName() throws SQLException {
        assertEquals("primusdb", meta.getUserName());
    }

    @Test
    void testReadOnly() throws SQLException {
        assertFalse(meta.isReadOnly());
    }

    @Test
    void testNullSorting() throws SQLException {
        assertFalse(meta.nullsAreSortedHigh());
        assertTrue(meta.nullsAreSortedLow());
        assertFalse(meta.nullsAreSortedAtStart());
        assertFalse(meta.nullsAreSortedAtEnd());
    }

    @Test
    void testIdentifierCase() throws SQLException {
        assertTrue(meta.supportsMixedCaseIdentifiers());
        assertFalse(meta.storesUpperCaseIdentifiers());
        assertFalse(meta.storesLowerCaseIdentifiers());
        assertTrue(meta.storesMixedCaseIdentifiers());
        assertTrue(meta.supportsMixedCaseQuotedIdentifiers());
        assertTrue(meta.storesMixedCaseQuotedIdentifiers());
    }

    @Test
    void testGetIdentifierQuoteString() throws SQLException {
        assertEquals("\"", meta.getIdentifierQuoteString());
    }

    @Test
    void testSQLKeywords() throws SQLException {
        assertEquals("", meta.getSQLKeywords());
    }

    @Test
    void testNumericFunctions() throws SQLException {
        assertEquals("", meta.getNumericFunctions());
    }

    @Test
    void testStringFunctions() throws SQLException {
        assertEquals("", meta.getStringFunctions());
    }

    @Test
    void testSystemFunctions() throws SQLException {
        assertEquals("", meta.getSystemFunctions());
    }

    @Test
    void testTimeDateFunctions() throws SQLException {
        assertEquals("", meta.getTimeDateFunctions());
    }

    @Test
    void testSearchStringEscape() throws SQLException {
        assertEquals("\\", meta.getSearchStringEscape());
    }

    @Test
    void testExtraNameCharacters() throws SQLException {
        assertEquals("", meta.getExtraNameCharacters());
    }

    @Test
    void testAllProceduresAreCallable() throws SQLException {
        assertFalse(meta.allProceduresAreCallable());
    }

    @Test
    void testAllTablesAreSelectable() throws SQLException {
        assertTrue(meta.allTablesAreSelectable());
    }

    @Test
    void testMaxLengths() throws SQLException {
        assertEquals(255, meta.getMaxColumnNameLength());
        assertEquals(255, meta.getMaxTableNameLength());
        assertEquals(255, meta.getMaxUserNameLength());
        assertEquals(0, meta.getMaxBinaryLiteralLength());
        assertEquals(0, meta.getMaxCharLiteralLength());
    }

    @Test
    void testDefaultTransactionIsolation() throws SQLException {
        assertEquals(Connection.TRANSACTION_NONE, meta.getDefaultTransactionIsolation());
    }

    @Test
    void testSupportsTransactionIsolationLevel() throws SQLException {
        assertTrue(meta.supportsTransactionIsolationLevel(Connection.TRANSACTION_NONE));
        assertFalse(meta.supportsTransactionIsolationLevel(Connection.TRANSACTION_READ_COMMITTED));
    }

    @Test
    void testSupportsTransactions() throws SQLException {
        assertFalse(meta.supportsTransactions());
    }

    @Test
    void testSupportsResultSetType() throws SQLException {
        assertTrue(meta.supportsResultSetType(ResultSet.TYPE_FORWARD_ONLY));
        assertFalse(meta.supportsResultSetType(ResultSet.TYPE_SCROLL_INSENSITIVE));
    }

    @Test
    void testSupportsResultSetConcurrency() throws SQLException {
        assertTrue(meta.supportsResultSetConcurrency(ResultSet.TYPE_FORWARD_ONLY, ResultSet.CONCUR_READ_ONLY));
        assertFalse(meta.supportsResultSetConcurrency(ResultSet.TYPE_FORWARD_ONLY, ResultSet.CONCUR_UPDATABLE));
    }

    @Test
    void testResultSetHoldability() throws SQLException {
        assertEquals(ResultSet.CLOSE_CURSORS_AT_COMMIT, meta.getResultSetHoldability());
        assertTrue(meta.supportsResultSetHoldability(ResultSet.CLOSE_CURSORS_AT_COMMIT));
        assertFalse(meta.supportsResultSetHoldability(ResultSet.HOLD_CURSORS_OVER_COMMIT));
    }

    @Test
    void testGetSQLStateType() throws SQLException {
        assertEquals(DatabaseMetaData.sqlStateSQL, meta.getSQLStateType());
    }

    @Test
    void testGetCatalogTerm() throws SQLException {
        assertEquals("catalog", meta.getCatalogTerm());
    }

    @Test
    void testGetCatalogSeparator() throws SQLException {
        assertEquals(".", meta.getCatalogSeparator());
    }

    @Test
    void testIsCatalogAtStart() throws SQLException {
        assertTrue(meta.isCatalogAtStart());
    }

    @Test
    void testGetProcedureTerm() throws SQLException {
        assertEquals("procedure", meta.getProcedureTerm());
    }

    @Test
    void testGetSchemaTerm() throws SQLException {
        assertEquals("schema", meta.getSchemaTerm());
    }

    @Test
    void testGetRowIdLifetime() throws SQLException {
        assertEquals(RowIdLifetime.ROWID_UNSUPPORTED, meta.getRowIdLifetime());
    }

    @Test
    void testGetSchemas() throws SQLException {
        assertNull(meta.getSchemas());
        assertNull(meta.getSchemas(null, null));
    }

    @Test
    void testGetCatalogs() throws SQLException {
        assertNull(meta.getCatalogs());
    }

    @Test
    void testGetTableTypes() throws SQLException {
        assertNull(meta.getTableTypes());
    }

    @Test
    void testGetConnection() throws SQLException {
        assertNotNull(meta.getConnection());
    }

    @Test
    void testLocatorsUpdateCopy() throws SQLException {
        assertFalse(meta.locatorsUpdateCopy());
    }

    @Test
    void testSupportsBatchUpdates() throws SQLException {
        assertFalse(meta.supportsBatchUpdates());
    }

    @Test
    void testSupportsSavepoints() throws SQLException {
        assertFalse(meta.supportsSavepoints());
    }

    @Test
    void testSupportsNamedParameters() throws SQLException {
        assertFalse(meta.supportsNamedParameters());
    }

    @Test
    void testSupportsMultipleOpenResults() throws SQLException {
        assertFalse(meta.supportsMultipleOpenResults());
    }

    @Test
    void testSupportsGetGeneratedKeys() throws SQLException {
        assertFalse(meta.supportsGetGeneratedKeys());
    }

    @Test
    void testSupportsStoredFunctionsUsingCallSyntax() throws SQLException {
        assertFalse(meta.supportsStoredFunctionsUsingCallSyntax());
    }

    @Test
    void testAutoCommitFailureClosesAllResultSets() throws SQLException {
        assertFalse(meta.autoCommitFailureClosesAllResultSets());
    }

    @Test
    void testGeneratedKeyAlwaysReturned() throws SQLException {
        assertFalse(meta.generatedKeyAlwaysReturned());
    }

    @Test
    void testSupportsStatementPooling() throws SQLException {
        assertFalse(meta.supportsStatementPooling());
    }

    @Test
    void testGetClientInfoProperties() throws SQLException {
        assertNull(meta.getClientInfoProperties());
    }

    @Test
    void testGetFunctions() throws SQLException {
        assertNull(meta.getFunctions(null, null, null));
        assertNull(meta.getFunctionColumns(null, null, null, null));
    }

    @Test
    void testGetPseudoColumns() throws SQLException {
        assertNull(meta.getPseudoColumns(null, null, null, null));
    }

    @Test
    void testGetProcedures() throws SQLException {
        assertNull(meta.getProcedures(null, null, null));
        assertNull(meta.getProcedureColumns(null, null, null, null));
    }

    @Test
    void testGetTables() throws SQLException {
        assertNull(meta.getTables(null, null, null, null));
    }

    @Test
    void testGetColumns() throws SQLException {
        assertNull(meta.getColumns(null, null, null, null));
    }

    @Test
    void testGetPrimaryKeys() throws SQLException {
        assertNull(meta.getPrimaryKeys(null, null, null));
    }

    @Test
    void testGetImportedExportedKeys() throws SQLException {
        assertNull(meta.getImportedKeys(null, null, null));
        assertNull(meta.getExportedKeys(null, null, null));
        assertNull(meta.getCrossReference(null, null, null, null, null, null));
    }

    @Test
    void testGetIndexInfo() throws SQLException {
        assertNull(meta.getIndexInfo(null, null, null, false, false));
    }

    @Test
    void testGetUDTs() throws SQLException {
        assertNull(meta.getUDTs(null, null, null, null));
    }

    @Test
    void testGetSuperTypes() throws SQLException {
        assertNull(meta.getSuperTypes(null, null, null));
    }

    @Test
    void testGetSuperTables() throws SQLException {
        assertNull(meta.getSuperTables(null, null, null));
    }

    @Test
    void testGetAttributes() throws SQLException {
        assertNull(meta.getAttributes(null, null, null, null));
    }

    @Test
    void testGetTypeInfo() throws SQLException {
        assertNull(meta.getTypeInfo());
    }

    @Test
    void testGetBestRowIdentifier() throws SQLException {
        assertNull(meta.getBestRowIdentifier(null, null, null, DatabaseMetaData.bestRowSession, false));
    }

    @Test
    void testGetVersionColumns() throws SQLException {
        assertNull(meta.getVersionColumns(null, null, null));
    }

    @Test
    void testGetColumnPrivileges() throws SQLException {
        assertNull(meta.getColumnPrivileges(null, null, null, null));
    }

    @Test
    void testGetTablePrivileges() throws SQLException {
        assertNull(meta.getTablePrivileges(null, null, null));
    }

    @Test
    void testUnwrap() throws SQLException {
        assertFalse(meta.isWrapperFor(PrimusDBDatabaseMetaData.class));
    }

    @Test
    void testSupportsAlterTable() throws SQLException {
        assertFalse(meta.supportsAlterTableWithAddColumn());
        assertFalse(meta.supportsAlterTableWithDropColumn());
    }

    @Test
    void testSupportsColumnAliasing() throws SQLException {
        assertTrue(meta.supportsColumnAliasing());
    }

    @Test
    void testNullPlusNonNullIsNull() throws SQLException {
        assertTrue(meta.nullPlusNonNullIsNull());
    }

    @Test
    void testSupportsConvert() throws SQLException {
        assertFalse(meta.supportsConvert());
        assertFalse(meta.supportsConvert(Types.INTEGER, Types.VARCHAR));
    }

    @Test
    void testSupportsTableCorrelationNames() throws SQLException {
        assertTrue(meta.supportsTableCorrelationNames());
        assertFalse(meta.supportsDifferentTableCorrelationNames());
    }

    @Test
    void testSupportsExpressionsInOrderBy() throws SQLException {
        assertFalse(meta.supportsExpressionsInOrderBy());
    }

    @Test
    void testSupportsOrderByUnrelated() throws SQLException {
        assertFalse(meta.supportsOrderByUnrelated());
    }

    @Test
    void testSupportsGroupBy() throws SQLException {
        assertFalse(meta.supportsGroupBy());
        assertFalse(meta.supportsGroupByUnrelated());
        assertFalse(meta.supportsGroupByBeyondSelect());
    }

    @Test
    void testSupportsLikeEscapeClause() throws SQLException {
        assertFalse(meta.supportsLikeEscapeClause());
    }

    @Test
    void testSupportsMultipleResultSets() throws SQLException {
        assertFalse(meta.supportsMultipleResultSets());
    }

    @Test
    void testSupportsMultipleTransactions() throws SQLException {
        assertFalse(meta.supportsMultipleTransactions());
    }

    @Test
    void testSupportsNonNullableColumns() throws SQLException {
        assertTrue(meta.supportsNonNullableColumns());
    }

    @Test
    void testSupportsMinimumSQLGrammar() throws SQLException {
        assertTrue(meta.supportsMinimumSQLGrammar());
    }

    @Test
    void testSupportsCoreSQLGrammar() throws SQLException {
        assertFalse(meta.supportsCoreSQLGrammar());
    }

    @Test
    void testSupportsExtendedSQLGrammar() throws SQLException {
        assertFalse(meta.supportsExtendedSQLGrammar());
    }

    @Test
    void testSupportsANSI92EntryLevelSQL() throws SQLException {
        assertFalse(meta.supportsANSI92EntryLevelSQL());
    }

    @Test
    void testSupportsANSI92IntermediateSQL() throws SQLException {
        assertFalse(meta.supportsANSI92IntermediateSQL());
    }

    @Test
    void testSupportsANSI92FullSQL() throws SQLException {
        assertFalse(meta.supportsANSI92FullSQL());
    }

    @Test
    void testSupportsIntegrityEnhancementFacility() throws SQLException {
        assertFalse(meta.supportsIntegrityEnhancementFacility());
    }

    @Test
    void testSupportsOuterJoins() throws SQLException {
        assertFalse(meta.supportsOuterJoins());
        assertFalse(meta.supportsFullOuterJoins());
        assertFalse(meta.supportsLimitedOuterJoins());
    }

    @Test
    void testSupportsSchemas() throws SQLException {
        assertFalse(meta.supportsSchemasInDataManipulation());
        assertFalse(meta.supportsSchemasInProcedureCalls());
        assertFalse(meta.supportsSchemasInTableDefinitions());
        assertFalse(meta.supportsSchemasInIndexDefinitions());
        assertFalse(meta.supportsSchemasInPrivilegeDefinitions());
    }

    @Test
    void testSupportsCatalogs() throws SQLException {
        assertFalse(meta.supportsCatalogsInDataManipulation());
        assertFalse(meta.supportsCatalogsInProcedureCalls());
        assertFalse(meta.supportsCatalogsInTableDefinitions());
        assertFalse(meta.supportsCatalogsInIndexDefinitions());
        assertFalse(meta.supportsCatalogsInPrivilegeDefinitions());
    }

    @Test
    void testSupportsPositionedDelete() throws SQLException {
        assertFalse(meta.supportsPositionedDelete());
    }

    @Test
    void testSupportsPositionedUpdate() throws SQLException {
        assertFalse(meta.supportsPositionedUpdate());
    }

    @Test
    void testSupportsSelectForUpdate() throws SQLException {
        assertFalse(meta.supportsSelectForUpdate());
    }

    @Test
    void testSupportsStoredProcedures() throws SQLException {
        assertFalse(meta.supportsStoredProcedures());
    }

    @Test
    void testSupportsSubqueries() throws SQLException {
        assertFalse(meta.supportsSubqueriesInComparisons());
        assertFalse(meta.supportsSubqueriesInExists());
        assertFalse(meta.supportsSubqueriesInIns());
        assertFalse(meta.supportsSubqueriesInQuantifieds());
        assertFalse(meta.supportsCorrelatedSubqueries());
    }

    @Test
    void testSupportsUnion() throws SQLException {
        assertFalse(meta.supportsUnion());
        assertFalse(meta.supportsUnionAll());
    }

    @Test
    void testSupportsOpenCursorsAcrossCommit() throws SQLException {
        assertFalse(meta.supportsOpenCursorsAcrossCommit());
        assertFalse(meta.supportsOpenCursorsAcrossRollback());
    }

    @Test
    void testSupportsOpenStatementsAcrossCommit() throws SQLException {
        assertFalse(meta.supportsOpenStatementsAcrossCommit());
        assertFalse(meta.supportsOpenStatementsAcrossRollback());
    }

    @Test
    void testOwnChangesAreVisible() throws SQLException {
        assertFalse(meta.ownUpdatesAreVisible(ResultSet.TYPE_FORWARD_ONLY));
        assertFalse(meta.ownDeletesAreVisible(ResultSet.TYPE_FORWARD_ONLY));
        assertFalse(meta.ownInsertsAreVisible(ResultSet.TYPE_FORWARD_ONLY));
    }

    @Test
    void testOthersChangesAreVisible() throws SQLException {
        assertFalse(meta.othersUpdatesAreVisible(ResultSet.TYPE_FORWARD_ONLY));
        assertFalse(meta.othersDeletesAreVisible(ResultSet.TYPE_FORWARD_ONLY));
        assertFalse(meta.othersInsertsAreVisible(ResultSet.TYPE_FORWARD_ONLY));
    }

    @Test
    void testChangesAreDetected() throws SQLException {
        assertFalse(meta.updatesAreDetected(ResultSet.TYPE_FORWARD_ONLY));
        assertFalse(meta.deletesAreDetected(ResultSet.TYPE_FORWARD_ONLY));
        assertFalse(meta.insertsAreDetected(ResultSet.TYPE_FORWARD_ONLY));
    }

    @Test
    void testDoesMaxRowSizeIncludeBlobs() throws SQLException {
        assertFalse(meta.doesMaxRowSizeIncludeBlobs());
    }
}
