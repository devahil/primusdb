use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use tempfile::TempDir;

fn find_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind ephemeral port");
    listener.local_addr().unwrap().port()
}

fn server_binary() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join("primusdb")
}

fn start_server(port: u16, data_dir: &TempDir) -> Child {
    Command::new(server_binary())
        .arg("server")
        .arg("start")
        .arg("--bind")
        .arg(format!("127.0.0.1:{}", port))
        .arg("--data-dir")
        .arg(data_dir.path())
        .arg("--log-level")
        .arg("error")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to start primusdb server")
}

fn wait_for_health(port: u16, timeout: Duration) -> bool {
    let client = Client::new();
    let start = Instant::now();
    while start.elapsed() < timeout {
        match client
            .get(format!("http://127.0.0.1:{}/health", port))
            .send()
        {
            Ok(resp) if resp.status().is_success() => return true,
            _ => std::thread::sleep(Duration::from_millis(100)),
        }
    }
    false
}

fn kill_process_and_children(mut child: Child) {
    let _ = child.kill();
    let _ = child.wait();
}

struct ServerProcess {
    child: Option<Child>,
    port: u16,
    _data_dir: TempDir,
}

impl ServerProcess {
    fn start(port: u16, data_dir: TempDir) -> Self {
        let child = Some(start_server(port, &data_dir));
        Self {
            child,
            port,
            _data_dir: data_dir,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.port, path)
    }

    fn client(&self) -> Client {
        Client::new()
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        if let Some(child) = self.child.take() {
            kill_process_and_children(child);
        }
    }
}

fn setup_server() -> ServerProcess {
    let port = find_free_port();
    let data_dir = TempDir::new().unwrap();
    let server = ServerProcess::start(port, data_dir);

    assert!(
        wait_for_health(port, Duration::from_secs(30)),
        "server did not become healthy within timeout"
    );

    server
}

#[test]
#[ignore]
fn test_crud_document_workflow() {
    let server = setup_server();

    let client = server.client();

    // Create a document record
    let create_resp = client
        .post(server.url("/api/v1/crud/document/test_docs"))
        .json(&serde_json::json!({"name": "Alice", "email": "alice@example.com", "age": 30}))
        .send()
        .unwrap();
    assert_eq!(create_resp.status(), 200);
    let create_body: serde_json::Value = create_resp.json().unwrap();
    assert_eq!(create_body["success"], true, "create should succeed");

    // Read document records
    let read_resp = client
        .get(server.url("/api/v1/crud/document/test_docs"))
        .send()
        .unwrap();
    assert_eq!(read_resp.status(), 200);
    let read_body: serde_json::Value = read_resp.json().unwrap();
    assert_eq!(read_body["success"], true);
    assert!(
        read_body["data"]["Select"].is_array(),
        "read should return Select array"
    );

    // Update the document
    let update_resp = client
        .put(server.url("/api/v1/crud/document/test_docs"))
        .json(&serde_json::json!({
            "data": {"age": 31},
            "conditions": {"name": "Alice"}
        }))
        .send()
        .unwrap();
    assert_eq!(update_resp.status(), 200);
    let update_body: serde_json::Value = update_resp.json().unwrap();
    assert_eq!(update_body["success"], true, "update should succeed");

    // Verify update
    let read_after_update = client
        .get(server.url("/api/v1/crud/document/test_docs"))
        .send()
        .unwrap();
    let read_after_update_body: serde_json::Value = read_after_update.json().unwrap();
    assert_eq!(read_after_update_body["success"], true);

    // Delete the document
    let delete_resp = client
        .delete(
            server.url("/api/v1/crud/document/test_docs?conditions=%7B%22name%22:%22Alice%22%7D"),
        )
        .send()
        .unwrap();
    assert_eq!(delete_resp.status(), 200);
    let delete_body: serde_json::Value = delete_resp.json().unwrap();
    assert_eq!(delete_body["success"], true, "delete should succeed");
}

#[test]
#[ignore]
fn test_crud_keyvalue_workflow() {
    let server = setup_server();
    let client = server.client();

    // Create a keyvalue record
    let create_resp = client
        .post(server.url("/api/v1/crud/keyvalue/test_kv"))
        .json(&serde_json::json!({"key": "user:1", "value": "Alice"}))
        .send()
        .unwrap();
    let create_status = create_resp.status();
    assert!(
        create_status == 200 || create_status == 201,
        "keyvalue create should return 2xx, got {}",
        create_status
    );

    // Read keyvalue records
    let read_resp = client
        .get(server.url("/api/v1/crud/keyvalue/test_kv"))
        .send()
        .unwrap();
    assert_eq!(read_resp.status(), 200);

    // Update keyvalue record
    let update_resp = client
        .put(server.url("/api/v1/crud/keyvalue/test_kv"))
        .json(&serde_json::json!({
            "data": {"value": "AliceUpdated"},
            "conditions": {"key": "user:1"}
        }))
        .send()
        .unwrap();
    assert_eq!(update_resp.status(), 200);

    // Delete keyvalue record
    let delete_resp = client
        .delete(server.url("/api/v1/crud/keyvalue/test_kv?conditions=%7B%22key%22:%22user:1%22%7D"))
        .send()
        .unwrap();
    assert_eq!(delete_resp.status(), 200);
}

#[test]
#[ignore]
fn test_error_empty_table_name() {
    let server = setup_server();
    let client = server.client();

    let resp = client
        .get(server.url("/api/v1/crud/document/"))
        .send()
        .unwrap();

    assert!(
        resp.status() == 404 || resp.status() == 400,
        "empty table name should return 4xx, got {}",
        resp.status()
    );
}

#[test]
#[ignore]
fn test_error_nonexistent_endpoint() {
    let server = setup_server();
    let client = server.client();

    let resp = client
        .get(server.url("/api/v1/nonexistent"))
        .send()
        .unwrap();

    assert_eq!(resp.status(), 404, "nonexistent endpoint should return 404");
}
