package com.primusdb.jdbc;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;
import java.sql.*;
import java.util.Properties;

class PrimusDBDriverTest {

    @Test
    void testAcceptsURL() throws SQLException {
        PrimusDBDriver driver = new PrimusDBDriver();
        assertTrue(driver.acceptsURL("jdbc:primusdb://localhost:8080/mydb"));
        assertTrue(driver.acceptsURL("jdbc:primusdb://host:1234"));
        assertFalse(driver.acceptsURL("jdbc:postgresql://localhost/db"));
        assertFalse(driver.acceptsURL(null));
    }

    @Test
    void testConnectReturnsConnection() throws SQLException {
        PrimusDBDriver driver = new PrimusDBDriver();
        Properties props = new Properties();
        props.setProperty("user", "testuser");
        props.setProperty("password", "testpass");
        Connection conn = driver.connect("jdbc:primusdb://localhost:8080/mydb", props);
        assertNotNull(conn);
        assertTrue(conn instanceof PrimusDBConnection);
        assertEquals("mydb", conn.getCatalog());
    }

    @Test
    void testConnectReturnsNullForUnsupportedURL() throws SQLException {
        PrimusDBDriver driver = new PrimusDBDriver();
        assertNull(driver.connect("jdbc:postgresql://localhost/db", new Properties()));
    }

    @Test
    void testConnectDefaultDatabase() throws SQLException {
        PrimusDBDriver driver = new PrimusDBDriver();
        Connection conn = driver.connect("jdbc:primusdb://localhost:8080", new Properties());
        assertEquals("default", conn.getCatalog());
    }

    @Test
    void testConnectDefaultPort() throws SQLException {
        PrimusDBDriver driver = new PrimusDBDriver();
        Connection conn = driver.connect("jdbc:primusdb://localhost", new Properties());
        assertEquals("default", conn.getCatalog());
    }

    @Test
    void testMajorMinorVersion() {
        PrimusDBDriver driver = new PrimusDBDriver();
        assertEquals(0, driver.getMajorVersion());
        assertEquals(1, driver.getMinorVersion());
    }

    @Test
    void testJDBCCompliant() {
        PrimusDBDriver driver = new PrimusDBDriver();
        assertTrue(driver.jdbcCompliant());
    }

    @Test
    void testGetPropertyInfo() throws SQLException {
        PrimusDBDriver driver = new PrimusDBDriver();
        DriverPropertyInfo[] info = driver.getPropertyInfo("jdbc:primusdb://localhost:8080", new Properties());
        assertNotNull(info);
        assertEquals(0, info.length);
    }

    @Test
    void testGetParentLogger() {
        PrimusDBDriver driver = new PrimusDBDriver();
        assertThrows(SQLFeatureNotSupportedException.class, driver::getParentLogger);
    }

    @Test
    void testDriverRegistered() throws SQLException {
        assertNotNull(DriverManager.getDriver("jdbc:primusdb://localhost:8080"));
    }
}
