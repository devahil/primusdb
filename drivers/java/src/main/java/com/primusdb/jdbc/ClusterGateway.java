package com.primusdb.jdbc;

import okhttp3.*;
import com.google.gson.*;
import java.io.IOException;
import java.util.List;

/**
 * Cluster Gateway utility for PrimusDB cluster management operations.
 *
 * Provides access to cluster status, nodes, route decisions, and metrics
 * via the PrimusDB HTTP API.
 */
public class ClusterGateway {

    private final PrimusDBConnection connection;
    private final OkHttpClient httpClient;
    private final Gson gson;

    public ClusterGateway(PrimusDBConnection connection) {
        this.connection = connection;
        this.httpClient = connection.getHttpClient();
        this.gson = new Gson();
    }

    public JsonObject clusterStatus() throws IOException {
        String url = connection.getBaseUrl() + "/api/v1/cluster/status";
        Request request = new Request.Builder().url(url).get().build();
        try (Response response = httpClient.newCall(request).execute()) {
            String body = response.body().string();
            return gson.fromJson(body, JsonObject.class);
        }
    }

    public JsonArray clusterNodes() throws IOException {
        String url = connection.getBaseUrl() + "/api/v1/cluster/nodes";
        Request request = new Request.Builder().url(url).get().build();
        try (Response response = httpClient.newCall(request).execute()) {
            String body = response.body().string();
            return gson.fromJson(body, JsonArray.class);
        }
    }

    public JsonObject routeRequest(String shardKey, String[] preferredNodes) throws IOException {
        String url = connection.getBaseUrl() + "/api/v1/cluster/route";
        JsonObject body = new JsonObject();
        if (shardKey != null) {
            body.addProperty("shard_key", shardKey);
        }
        if (preferredNodes != null) {
            JsonArray nodes = new JsonArray();
            for (String node : preferredNodes) {
                nodes.add(node);
            }
            body.add("preferred_nodes", nodes);
        }
        RequestBody reqBody = RequestBody.create(body.toString(), MediaType.parse("application/json"));
        Request request = new Request.Builder().url(url).post(reqBody).build();
        try (Response response = httpClient.newCall(request).execute()) {
            String respBody = response.body().string();
            return gson.fromJson(respBody, JsonObject.class);
        }
    }

    public JsonObject clusterMetrics() throws IOException {
        String url = connection.getBaseUrl() + "/api/v1/cluster/metrics";
        Request request = new Request.Builder().url(url).get().build();
        try (Response response = httpClient.newCall(request).execute()) {
            String body = response.body().string();
            return gson.fromJson(body, JsonObject.class);
        }
    }

    public JsonObject federationStatus() throws IOException {
        String url = connection.getBaseUrl() + "/api/v1/federation/status";
        Request request = new Request.Builder().url(url).get().build();
        try (Response response = httpClient.newCall(request).execute()) {
            String body = response.body().string();
            return gson.fromJson(body, JsonObject.class);
        }
    }

    public JsonObject federationClusters() throws IOException {
        String url = connection.getBaseUrl() + "/api/v1/federation/clusters";
        Request request = new Request.Builder().url(url).get().build();
        try (Response response = httpClient.newCall(request).execute()) {
            String body = response.body().string();
            return gson.fromJson(body, JsonObject.class);
        }
    }

    public JsonObject federationDomains() throws IOException {
        String url = connection.getBaseUrl() + "/api/v1/federation/domains";
        Request request = new Request.Builder().url(url).get().build();
        try (Response response = httpClient.newCall(request).execute()) {
            String body = response.body().string();
            return gson.fromJson(body, JsonObject.class);
        }
    }

    public JsonObject createDataDomain(String name, String description, String replicationMode, List<String> storageTypes, List<String> collections, List<String> tables, List<String> memberClusters) throws IOException {
        String url = connection.getBaseUrl() + "/api/v1/federation/domains";
        JsonObject body = new JsonObject();
        body.addProperty("name", name);
        if (description != null) {
            body.addProperty("description", description);
        }
        if (replicationMode != null) {
            body.addProperty("replication_mode", replicationMode);
        }
        if (storageTypes != null) {
            JsonArray arr = new JsonArray();
            for (String s : storageTypes) {
                arr.add(s);
            }
            body.add("storage_types", arr);
        }
        if (collections != null) {
            JsonArray arr = new JsonArray();
            for (String s : collections) {
                arr.add(s);
            }
            body.add("collections", arr);
        }
        if (tables != null) {
            JsonArray arr = new JsonArray();
            for (String s : tables) {
                arr.add(s);
            }
            body.add("tables", arr);
        }
        if (memberClusters != null) {
            JsonArray arr = new JsonArray();
            for (String s : memberClusters) {
                arr.add(s);
            }
            body.add("member_clusters", arr);
        }
        RequestBody reqBody = RequestBody.create(body.toString(), MediaType.parse("application/json"));
        Request request = new Request.Builder().url(url).post(reqBody).build();
        try (Response response = httpClient.newCall(request).execute()) {
            String respBody = response.body().string();
            return gson.fromJson(respBody, JsonObject.class);
        }
    }

    public JsonObject joinDomain(String name, List<String> collections, List<String> storageTypes, String replicationMode) throws IOException {
        String url = connection.getBaseUrl() + "/api/v1/federation/domains/" + name + "/join";
        JsonObject body = new JsonObject();
        if (collections != null) {
            JsonArray arr = new JsonArray();
            for (String s : collections) {
                arr.add(s);
            }
            body.add("collections", arr);
        }
        if (storageTypes != null) {
            JsonArray arr = new JsonArray();
            for (String s : storageTypes) {
                arr.add(s);
            }
            body.add("storage_types", arr);
        }
        if (replicationMode != null) {
            body.addProperty("replication_mode", replicationMode);
        }
        RequestBody reqBody = RequestBody.create(body.toString(), MediaType.parse("application/json"));
        Request request = new Request.Builder().url(url).post(reqBody).build();
        try (Response response = httpClient.newCall(request).execute()) {
            String respBody = response.body().string();
            return gson.fromJson(respBody, JsonObject.class);
        }
    }

    public JsonObject leaveDomain(String name) throws IOException {
        String url = connection.getBaseUrl() + "/api/v1/federation/domains/" + name + "/leave";
        JsonObject body = new JsonObject();
        body.addProperty("name", name);
        RequestBody reqBody = RequestBody.create(body.toString(), MediaType.parse("application/json"));
        Request request = new Request.Builder().url(url).post(reqBody).build();
        try (Response response = httpClient.newCall(request).execute()) {
            String respBody = response.body().string();
            return gson.fromJson(respBody, JsonObject.class);
        }
    }

    public JsonObject balanceDomain(String name) throws IOException {
        String url = connection.getBaseUrl() + "/api/v1/federation/domains/" + name + "/balance";
        JsonObject body = new JsonObject();
        body.addProperty("name", name);
        RequestBody reqBody = RequestBody.create(body.toString(), MediaType.parse("application/json"));
        Request request = new Request.Builder().url(url).post(reqBody).build();
        try (Response response = httpClient.newCall(request).execute()) {
            String respBody = response.body().string();
            return gson.fromJson(respBody, JsonObject.class);
        }
    }

    public JsonObject federationMetrics() throws IOException {
        String url = connection.getBaseUrl() + "/api/v1/federation/metrics";
        Request request = new Request.Builder().url(url).get().build();
        try (Response response = httpClient.newCall(request).execute()) {
            String body = response.body().string();
            return gson.fromJson(body, JsonObject.class);
        }
    }
}
