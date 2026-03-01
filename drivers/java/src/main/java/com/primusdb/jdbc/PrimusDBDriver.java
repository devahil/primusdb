package com.primusdb.jdbc;

/*
 * PrimusDB JDBC Driver
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.2.0-alpha - Full JDBC implementation with OkHttp client
 */

import java.sql.*;
import java.util.Properties;
import java.util.logging.Logger;

/**
 * JDBC Driver for PrimusDB
 */
public class PrimusDBDriver implements Driver {

    private static final String URL_PREFIX = "jdbc:primusdb://";

    static {
        try {
            DriverManager.registerDriver(new PrimusDBDriver());
        } catch (SQLException e) {
            throw new RuntimeException("Failed to register PrimusDB JDBC driver", e);
        }
    }

    @Override
    public Connection connect(String url, Properties info) throws SQLException {
        if (!acceptsURL(url)) {
            return null;
        }

        // Parse URL: jdbc:primusdb://host:port/database
        String cleanUrl = url.substring(URL_PREFIX.length());
        String[] parts = cleanUrl.split("/");
        String hostPort = parts[0];
        String database = parts.length > 1 ? parts[1] : "default";

        String[] hostPortParts = hostPort.split(":");
        String host = hostPortParts[0];
        int port = hostPortParts.length > 1 ? Integer.parseInt(hostPortParts[1]) : 8080;

        String username = info.getProperty("user");
        String password = info.getProperty("password");

        return new PrimusDBConnection(host, port, database, username, password);
    }

    @Override
    public boolean acceptsURL(String url) throws SQLException {
        return url != null && url.startsWith(URL_PREFIX);
    }

    @Override
    public DriverPropertyInfo[] getPropertyInfo(String url, Properties info) throws SQLException {
        return new DriverPropertyInfo[0];
    }

    @Override
    public int getMajorVersion() {
        return 0;
    }

    @Override
    public int getMinorVersion() {
        return 1;
    }

    @Override
    public boolean jdbcCompliant() {
        return true;
    }

    @Override
    public Logger getParentLogger() throws SQLFeatureNotSupportedException {
        throw new SQLFeatureNotSupportedException("Parent logger not supported");
    }
}