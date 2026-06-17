import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.net.http.HttpRequest.BodyPublishers;
import java.time.Duration;

public class BasicExample {
    private static final String BASE_URL = "http://localhost:8080";
    private static final HttpClient client = HttpClient.newBuilder()
            .connectTimeout(Duration.ofSeconds(5))
            .build();

    public static void main(String[] args) {
        System.out.println("=== PrimusDB Java Example ===");

        try {
            // Check health
            String health = get("/health");
            System.out.println("Health: " + health);
        } catch (Exception e) {
            System.err.println("Error connecting to PrimusDB: " + e.getMessage());
            System.exit(1);
        }

        try {
            // Get version
            String version = get("/version");
            System.out.println("Version: " + version);
        } catch (Exception e) {
            System.err.println("Error fetching version: " + e.getMessage());
        }

        try {
            // Create a record
            String record = "{\n" +
                    "    \"collection\": \"users\",\n" +
                    "    \"data\": {\n" +
                    "        \"name\": \"Alice\",\n" +
                    "        \"email\": \"alice@example.com\"\n" +
                    "    }\n" +
                    "}";
            String created = post("/records", record);
            System.out.println("Created record: " + created);
        } catch (Exception e) {
            System.err.println("Error creating record: " + e.getMessage());
        }

        try {
            // Query records
            String records = get("/records/users");
            System.out.println("Records: " + records);
        } catch (Exception e) {
            System.err.println("Error querying records: " + e.getMessage());
        }
    }

    private static String get(String path) throws Exception {
        HttpRequest req = HttpRequest.newBuilder()
                .uri(URI.create(BASE_URL + path))
                .timeout(Duration.ofSeconds(5))
                .GET()
                .build();
        HttpResponse<String> resp = client.send(req, HttpResponse.BodyHandlers.ofString());
        return resp.body();
    }

    private static String post(String path, String body) throws Exception {
        HttpRequest req = HttpRequest.newBuilder()
                .uri(URI.create(BASE_URL + path))
                .timeout(Duration.ofSeconds(5))
                .header("Content-Type", "application/json")
                .POST(BodyPublishers.ofString(body))
                .build();
        HttpResponse<String> resp = client.send(req, HttpResponse.BodyHandlers.ofString());
        return resp.body();
    }
}
