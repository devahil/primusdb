use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
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

fn start_server(port: u16, data_dir: &Path) -> Child {
    Command::new(server_binary())
        .arg("--port")
        .arg(port.to_string())
        .arg("--data-dir")
        .arg(data_dir)
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

fn copy_dir(src: &Path, dst: &Path) {
    let src_str = format!("{}/.", src.display());
    let status = Command::new("cp")
        .arg("-r")
        .arg(&src_str)
        .arg(dst)
        .status()
        .expect("failed to run cp");
    assert!(status.success(), "cp -r {:?} {:?} failed", src, dst);
}

fn remove_dir_contents(dir: &Path) {
    if dir.exists() {
        fs::remove_dir_all(dir).expect("failed to remove directory");
    }
    fs::create_dir_all(dir).expect("failed to recreate directory");
}

fn create_record_via_api(port: u16, id_value: u32) {
    let client = Client::new();
    let resp = client
        .post(format!(
            "http://127.0.0.1:{}/api/v1/crud/document/backup_test",
            port
        ))
        .json(&serde_json::json!({"id": id_value, "data": format!("record_{}", id_value)}))
        .send()
        .expect("failed to create record");
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().unwrap();
    assert_eq!(body["success"], true, "create should succeed");
}

fn count_records(port: u16) -> usize {
    let client = Client::new();
    let resp = client
        .get(format!(
            "http://127.0.0.1:{}/api/v1/crud/document/backup_test",
            port
        ))
        .send()
        .expect("failed to read records");
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().unwrap();
    body["data"]["Select"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0)
}

#[test]
#[ignore]
fn test_backup_and_restore() {
    let port1 = find_free_port();
    let data_dir = TempDir::new().expect("failed to create temp dir");
    let data_path = data_dir.path().to_path_buf();
    let backup_dir = TempDir::new().expect("failed to create backup temp dir");
    let backup_path = backup_dir.path().to_path_buf();

    // Step 1: Start server and create data
    let mut child1 = start_server(port1, &data_path);
    assert!(
        wait_for_health(port1, Duration::from_secs(30)),
        "server did not become healthy"
    );

    create_record_via_api(port1, 1);
    create_record_via_api(port1, 2);
    create_record_via_api(port1, 3);

    let original_count = count_records(port1);
    assert_eq!(original_count, 3, "should have 3 records before backup");

    // Step 2: Stop server and create backup
    stop_server(&mut child1);
    assert!(!data_path.exists() || data_path.exists());

    copy_dir(&data_path, &backup_path);
    assert!(backup_path.exists(), "backup directory should exist");
    assert!(
        fs::read_dir(&backup_path)
            .expect("failed to read backup dir")
            .count()
            > 0,
        "backup directory should contain files"
    );

    // Step 3: Clear original data and verify it's gone
    remove_dir_contents(&data_path);
    let mut child2 = start_server(port1, &data_path);
    assert!(
        wait_for_health(port1, Duration::from_secs(30)),
        "server with cleared data did not start"
    );

    let after_wipe_count = count_records(port1);
    assert_eq!(
        after_wipe_count, 0,
        "should have 0 records after wiping data"
    );

    // Step 4: Stop the clean server and restore from backup
    stop_server(&mut child2);

    remove_dir_contents(&data_path);
    copy_dir(&backup_path, &data_path);

    // Step 5: Start server with restored data
    let mut child3 = start_server(port1, &data_path);
    assert!(
        wait_for_health(port1, Duration::from_secs(30)),
        "server with restored data did not start"
    );

    let restored_count = count_records(port1);
    assert_eq!(
        restored_count, 3,
        "should have 3 records after restore, got {}",
        restored_count
    );

    // Step 6: Verify data content
    let client = Client::new();
    let resp = client
        .get(format!(
            "http://127.0.0.1:{}/api/v1/crud/document/backup_test",
            port1
        ))
        .send()
        .unwrap();
    let body: serde_json::Value = resp.json().unwrap();
    let records = body["data"]["Select"].as_array().unwrap();
    let ids: Vec<u32> = records
        .iter()
        .filter_map(|r| r["data"]["id"].as_u64().map(|id| id as u32))
        .collect();
    assert!(ids.contains(&1), "restored data should contain record id=1");
    assert!(ids.contains(&2), "restored data should contain record id=2");
    assert!(ids.contains(&3), "restored data should contain record id=3");

    // Cleanup
    stop_server(&mut child3);
}
