package com.primusdb.jdbc;

import com.google.gson.Gson;
import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import okhttp3.*;

import java.io.IOException;
import java.sql.SQLException;

public class GovernorGateway {
    private final PrimusDBConnection connection;
    private final OkHttpClient httpClient;
    private final Gson gson;
    private final String baseUrl;

    private static final MediaType JSON = MediaType.parse("application/json; charset=utf-8");

    public GovernorGateway(PrimusDBConnection connection) {
        this.connection = connection;
        this.httpClient = new OkHttpClient();
        this.gson = new Gson();
        this.baseUrl = "http://" + connection.getHost() + ":" + connection.getPort() + "/api/v1";
    }

    public JsonObject startExecution(String namespace, String workloadType, String user, String role)
            throws SQLException {
        try {
            JsonObject payload = new JsonObject();
            payload.addProperty("namespace", namespace);
            payload.addProperty("workload_type", workloadType);
            if (user != null) payload.addProperty("user", user);
            if (role != null) payload.addProperty("role", role);

            RequestBody body = RequestBody.create(payload.toString(), JSON);
            Request request = new Request.Builder()
                    .url(baseUrl + "/governor/executions/start")
                    .post(body)
                    .build();

            try (Response response = httpClient.newCall(request).execute()) {
                JsonObject json = gson.fromJson(response.body().string(), JsonObject.class);
                if (json.get("success").getAsBoolean()) {
                    return json.getAsJsonObject("data");
                }
                throw new SQLException(json.get("error").getAsString());
            }
        } catch (IOException e) {
            throw new SQLException("Governor start execution failed: " + e.getMessage());
        }
    }

    public void finishExecution(String executionId) throws SQLException {
        try {
            RequestBody body = RequestBody.create("{}", JSON);
            Request request = new Request.Builder()
                    .url(baseUrl + "/governor/executions/" + executionId + "/finish")
                    .post(body)
                    .build();

            try (Response response = httpClient.newCall(request).execute()) {
                JsonObject json = gson.fromJson(response.body().string(), JsonObject.class);
                if (!json.get("success").getAsBoolean()) {
                    throw new SQLException(json.get("error").getAsString());
                }
            }
        } catch (IOException e) {
            throw new SQLException("Governor finish execution failed: " + e.getMessage());
        }
    }

    public JsonObject checkLimit(String executionId, String checkType, long value) throws SQLException {
        try {
            JsonObject payload = new JsonObject();
            payload.addProperty("check_type", checkType);
            payload.addProperty("value", value);

            RequestBody body = RequestBody.create(payload.toString(), JSON);
            Request request = new Request.Builder()
                    .url(baseUrl + "/governor/executions/" + executionId + "/check")
                    .post(body)
                    .build();

            try (Response response = httpClient.newCall(request).execute()) {
                JsonObject json = gson.fromJson(response.body().string(), JsonObject.class);
                if (json.get("success").getAsBoolean()) {
                    return json.getAsJsonObject("data");
                }
                throw new SQLException(json.get("error").getAsString());
            }
        } catch (IOException e) {
            throw new SQLException("Governor check limit failed: " + e.getMessage());
        }
    }

    public JsonObject status() throws SQLException {
        try {
            Request request = new Request.Builder()
                    .url(baseUrl + "/governor/status")
                    .get()
                    .build();

            try (Response response = httpClient.newCall(request).execute()) {
                JsonObject json = gson.fromJson(response.body().string(), JsonObject.class);
                if (json.get("success").getAsBoolean()) {
                    return json.getAsJsonObject("data");
                }
                throw new SQLException(json.get("error").getAsString());
            }
        } catch (IOException e) {
            throw new SQLException("Governor status failed: " + e.getMessage());
        }
    }

    public JsonObject metrics() throws SQLException {
        try {
            Request request = new Request.Builder()
                    .url(baseUrl + "/governor/metrics")
                    .get()
                    .build();

            try (Response response = httpClient.newCall(request).execute()) {
                JsonObject json = gson.fromJson(response.body().string(), JsonObject.class);
                if (json.get("success").getAsBoolean()) {
                    return json.getAsJsonObject("data");
                }
                throw new SQLException(json.get("error").getAsString());
            }
        } catch (IOException e) {
            throw new SQLException("Governor metrics failed: " + e.getMessage());
        }
    }

    public JsonArray listExecutions() throws SQLException {
        try {
            Request request = new Request.Builder()
                    .url(baseUrl + "/governor/executions")
                    .get()
                    .build();

            try (Response response = httpClient.newCall(request).execute()) {
                JsonObject json = gson.fromJson(response.body().string(), JsonObject.class);
                if (json.get("success").getAsBoolean()) {
                    return json.getAsJsonArray("data");
                }
                throw new SQLException(json.get("error").getAsString());
            }
        } catch (IOException e) {
            throw new SQLException("Governor list executions failed: " + e.getMessage());
        }
    }

    public JsonArray listViolations() throws SQLException {
        try {
            Request request = new Request.Builder()
                    .url(baseUrl + "/governor/violations")
                    .get()
                    .build();

            try (Response response = httpClient.newCall(request).execute()) {
                JsonObject json = gson.fromJson(response.body().string(), JsonObject.class);
                if (json.get("success").getAsBoolean()) {
                    return json.getAsJsonArray("data");
                }
                throw new SQLException(json.get("error").getAsString());
            }
        } catch (IOException e) {
            throw new SQLException("Governor list violations failed: " + e.getMessage());
        }
    }

    public JsonArray policies() throws SQLException {
        try {
            Request request = new Request.Builder()
                    .url(baseUrl + "/governor/policies")
                    .get()
                    .build();

            try (Response response = httpClient.newCall(request).execute()) {
                JsonObject json = gson.fromJson(response.body().string(), JsonObject.class);
                if (json.get("success").getAsBoolean()) {
                    return json.getAsJsonArray("data");
                }
                throw new SQLException(json.get("error").getAsString());
            }
        } catch (IOException e) {
            throw new SQLException("Governor policies failed: " + e.getMessage());
        }
    }

    public void updatePolicy(String name, JsonObject limits, String action, String scope) throws SQLException {
        try {
            JsonObject payload = new JsonObject();
            payload.addProperty("name", name);
            payload.add("limits", limits);
            payload.addProperty("action", action);
            payload.addProperty("scope", scope);

            RequestBody body = RequestBody.create(payload.toString(), JSON);
            Request request = new Request.Builder()
                    .url(baseUrl + "/governor/policies/update")
                    .post(body)
                    .build();

            try (Response response = httpClient.newCall(request).execute()) {
                JsonObject json = gson.fromJson(response.body().string(), JsonObject.class);
                if (!json.get("success").getAsBoolean()) {
                    throw new SQLException(json.get("error").getAsString());
                }
            }
        } catch (IOException e) {
            throw new SQLException("Governor update policy failed: " + e.getMessage());
        }
    }
}
