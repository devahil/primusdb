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
    path.join("primusdb-server")
}

fn start_server(port: u16, data_dir: &TempDir) -> Child {
    Command::new(server_binary())
        .arg("--port")
        .arg(port.to_string())
        .arg("--data-dir")
        .arg(data_dir.path())
        .arg("--log-level")
        .arg("error")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to start primusdb-server")
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

fn stop_server(child: &mut Child) {
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
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[test]
#[ignore]
fn test_server_health_endpoint() {
    let port = find_free_port();
    let data_dir = TempDir::new().unwrap();
    let server = ServerProcess::start(port, data_dir);

    assert!(
        wait_for_health(port, Duration::from_secs(30)),
        "server did not become healthy within timeout"
    );

    let resp = Client::new().get(server.url("/health")).send().unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().unwrap();
    assert_eq!(body["success"], true);
    assert_eq!(body["data"]["status"], "healthy");
    assert!(body["data"]["version"].as_str().is_some());
}

#[test]
#[ignore]
fn test_server_status_endpoint() {
    let port = find_free_port();
    let data_dir = TempDir::new().unwrap();
    let server = ServerProcess::start(port, data_dir);

    assert!(
        wait_for_health(port, Duration::from_secs(30)),
        "server did not become healthy within timeout"
    );

    let resp = Client::new().get(server.url("/status")).send().unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().unwrap();
    assert_eq!(body["success"], true);
    assert_eq!(body["data"]["status"], "running");
    assert!(body["data"]["version"].as_str().is_some());
}

#[test]
#[ignore]
fn test_server_metrics_endpoint() {
    let port = find_free_port();
    let data_dir = TempDir::new().unwrap();
    let server = ServerProcess::start(port, data_dir);

    assert!(
        wait_for_health(port, Duration::from_secs(30)),
        "server did not become healthy within timeout"
    );

    let resp = Client::new().get(server.url("/metrics")).send().unwrap();
    assert_eq!(resp.status(), 200);

    let body = resp.text().unwrap();
    assert!(
        body.contains("primusdb_up"),
        "metrics should contain primusdb_up"
    );
    assert!(
        body.contains("primusdb_version"),
        "metrics should contain primusdb_version"
    );
}

#[test]
#[ignore]
fn test_server_graceful_shutdown() {
    let port = find_free_port();
    let data_dir = TempDir::new().unwrap();
    let mut child = start_server(port, &data_dir);

    assert!(
        wait_for_health(port, Duration::from_secs(30)),
        "server did not become healthy within timeout"
    );

    stop_server(&mut child);
}
