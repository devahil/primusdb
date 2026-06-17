const BASE_URL = "http://localhost:8080";

interface HealthResponse {
  status: string;
  [key: string]: unknown;
}

interface MyRecord {
  id?: string;
  collection: string;
  data: Record<string, unknown>;
}

async function main(): Promise<void> {
  console.log("=== PrimusDB Node.js Example ===");

  try {
    // Check health
    const healthResp = await fetch(`${BASE_URL}/health`);
    const health: HealthResponse = await healthResp.json();
    console.log("Health:", JSON.stringify(health));
  } catch (e) {
    console.error("Error connecting to PrimusDB:", e);
    return;
  }

  try {
    // Get version
    const versionResp = await fetch(`${BASE_URL}/version`);
    const version = await versionResp.json();
    console.log("Version:", JSON.stringify(version));
  } catch (e) {
    console.error("Error fetching version:", e);
  }

  try {
    // Create a record
    const record: MyRecord = {
      collection: "users",
      data: {
        name: "Alice",
        email: "alice@example.com",
      },
    };
    const createResp = await fetch(`${BASE_URL}/records`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(record),
    });
    const created = await createResp.json();
    console.log("Created record:", JSON.stringify(created));
  } catch (e) {
    console.error("Error creating record:", e);
  }

  try {
    // Query records
    const queryResp = await fetch(`${BASE_URL}/records/users`);
    const records = await queryResp.json();
    console.log("Records:", JSON.stringify(records));
  } catch (e) {
    console.error("Error querying records:", e);
  }
}

main();
