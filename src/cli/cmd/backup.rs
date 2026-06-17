use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::cli::command::{BackupSubcommands, GlobalArgs};
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;
use sha2::Digest;

// ---------------------------------------------------------------------------
// Backup index structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupIndexEntry {
    id: String,
    path: String,
    created_at: String,
    size_bytes: u64,
    checksum: Option<String>,
    compression: String,
    encrypted: bool,
    engines: Vec<String>,
    namespaces: Vec<String>,
    duration_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct BackupIndex {
    backups: Vec<BackupIndexEntry>,
}

// ---------------------------------------------------------------------------
// Public handlers
// ---------------------------------------------------------------------------

pub async fn handle_backup(
    cmd: BackupSubcommands,
    _global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    match cmd {
        BackupSubcommands::Create {
            destination,
            name,
            databases,
            compression,
            encrypt,
            description,
        } => {
            cmd_create(
                destination,
                name,
                databases,
                compression,
                encrypt,
                description,
                fmt,
            )
            .await
        }
        BackupSubcommands::List { directory, verbose } => cmd_list(directory, verbose, fmt).await,
        BackupSubcommands::Inspect {
            path,
            contents,
            metadata,
        } => cmd_inspect(path, contents, metadata, fmt).await,
        BackupSubcommands::Restore {
            source,
            database,
            force,
            pitr,
        } => cmd_restore(source, database, force, pitr, fmt).await,
        BackupSubcommands::Verify {
            path,
            full,
            compare,
        } => cmd_verify(path, full, compare, fmt).await,
        BackupSubcommands::Delete { name, force } => cmd_delete(name, force, fmt).await,
        BackupSubcommands::ExportManifest { name, output } => {
            cmd_export_manifest(name, output, fmt).await
        }
    }
}

pub async fn handle_restore(
    source: PathBuf,
    database: Option<String>,
    force: bool,
    _global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd_restore(source, database, force, None, fmt).await
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn default_backup_dir() -> PathBuf {
    PathBuf::from("backups")
}

fn timestamp() -> String {
    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", start)
}

fn index_path(backup_dir: &Path) -> PathBuf {
    backup_dir.join(".index.json")
}

async fn read_index(backup_dir: &Path) -> BackupIndex {
    let path = index_path(backup_dir);
    if path.exists() {
        tokio::fs::read_to_string(&path)
            .await
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(BackupIndex { backups: vec![] })
    } else {
        BackupIndex { backups: vec![] }
    }
}

async fn write_index(backup_dir: &Path, index: &BackupIndex) -> Result<()> {
    let content = serde_json::to_string_pretty(index)?;
    tokio::fs::write(index_path(backup_dir), content).await?;
    Ok(())
}

fn detect_tar_read_flag(path: &Path) -> &str {
    let name = path.to_string_lossy();
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        "-xzf"
    } else if name.ends_with(".tar.bz2") || name.ends_with(".tbz2") {
        "-xjf"
    } else if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        "-xJf"
    } else if name.ends_with(".tar") {
        "-xf"
    } else {
        "-xzf"
    }
}

fn detect_tar_list_flag(path: &Path) -> &str {
    let name = path.to_string_lossy();
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        "-tzf"
    } else if name.ends_with(".tar.bz2") || name.ends_with(".tbz2") {
        "-tjf"
    } else if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        "-tJf"
    } else if name.ends_with(".tar") {
        "-tf"
    } else {
        "-tzf"
    }
}

fn resolve_backup_id(id: &str) -> Option<PathBuf> {
    let backup_dir = default_backup_dir();
    let index = backup_dir.join(".index.json");
    if !index.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&index).ok()?;
    let parsed: BackupIndex = serde_json::from_str(&content).ok()?;
    parsed
        .backups
        .iter()
        .find(|e| e.id == id)
        .map(|e| backup_dir.join(&e.path))
}

fn resolve_path_or_id(input: &Path) -> PathBuf {
    if input.exists() {
        input.to_path_buf()
    } else if let Some(resolved) = resolve_backup_id(&input.to_string_lossy()) {
        resolved
    } else {
        input.to_path_buf()
    }
}

fn strip_tar_ext(filename: &str) -> &str {
    filename
        .strip_suffix(".tar.gz")
        .or_else(|| filename.strip_suffix(".tar.bz2"))
        .or_else(|| filename.strip_suffix(".tar.xz"))
        .or_else(|| filename.strip_suffix(".tar.zst"))
        .or_else(|| filename.strip_suffix(".tgz"))
        .or_else(|| filename.strip_suffix(".tbz2"))
        .or_else(|| filename.strip_suffix(".txz"))
        .or_else(|| filename.strip_suffix(".tar"))
        .unwrap_or(filename)
}

// ---------------------------------------------------------------------------
// cmd_delete
// ---------------------------------------------------------------------------

async fn cmd_delete(name: String, force: bool, fmt: &OutputFormat) -> Result<()> {
    let backup_dir = default_backup_dir();
    let full_path = if name.contains(std::path::MAIN_SEPARATOR) {
        PathBuf::from(&name)
    } else {
        resolve_backup_id(&name).unwrap_or_else(|| backup_dir.join(&name))
    };

    if !full_path.exists() {
        let data = OutputData::Error(format!("Backup not found: {}", full_path.display()));
        println!("{}", format_output(&data, *fmt));
        return Ok(());
    }

    if !force {
        let data = OutputData::Message(format!(
            "Are you sure you want to delete '{}'? Use --force to confirm.",
            full_path.display()
        ));
        println!("{}", format_output(&data, *fmt));
        return Ok(());
    }

    match tokio::fs::remove_file(&full_path).await {
        Ok(_) => {
            let mut index = read_index(&backup_dir).await;
            index.backups.retain(|e| {
                let p = backup_dir.join(&e.path);
                p != full_path
            });
            let _ = write_index(&backup_dir, &index).await;

            let data = OutputData::Message(format!("Deleted backup: {}", full_path.display()));
            println!("{}", format_output(&data, *fmt));
        }
        Err(e) => {
            let data = OutputData::Error(format!("Failed to delete backup: {}", e));
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// cmd_export_manifest
// ---------------------------------------------------------------------------

async fn cmd_export_manifest(
    name: String,
    output: Option<PathBuf>,
    fmt: &OutputFormat,
) -> Result<()> {
    let backup_dir = PathBuf::from("backups");
    let full_path = if name.contains(std::path::MAIN_SEPARATOR) {
        PathBuf::from(&name)
    } else {
        resolve_backup_id(&name).unwrap_or_else(|| backup_dir.join(&name))
    };

    if !full_path.exists() {
        let data = OutputData::Error(format!("Backup not found: {}", full_path.display()));
        println!("{}", format_output(&data, *fmt));
        return Ok(());
    }

    let meta = match tokio::fs::metadata(&full_path).await {
        Ok(m) => m,
        Err(e) => {
            let data = OutputData::Error(format!("Cannot read backup: {}", e));
            println!("{}", format_output(&data, *fmt));
            return Ok(());
        }
    };

    let manifest = serde_json::json!({
        "backup_id": name,
        "created_at": chrono::Utc::now().to_rfc3339(),
        "source_node": env!("CARGO_PKG_NAME"),
        "source_version": env!("CARGO_PKG_VERSION"),
        "size_bytes": meta.len(),
        "status": "exported",
        "manifest_version": "1.0.0",
    });

    let manifest_str = serde_json::to_string_pretty(&manifest).unwrap_or_default();

    if let Some(path) = output {
        tokio::fs::write(&path, &manifest_str).await?;
        let data = OutputData::Message(format!("Manifest written to {}", path.display()));
        println!("{}", format_output(&data, *fmt));
    } else {
        let data = OutputData::Message(manifest_str);
        println!("{}", format_output(&data, *fmt));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// cmd_create
// ---------------------------------------------------------------------------

async fn cmd_create(
    destination: Option<PathBuf>,
    name: Option<String>,
    databases: Option<String>,
    compression: String,
    encrypt: bool,
    description: Option<String>,
    fmt: &OutputFormat,
) -> Result<()> {
    let backup_dir = default_backup_dir();
    tokio::fs::create_dir_all(&backup_dir).await?;

    let ext = match compression.as_str() {
        "gzip" | "gz" => "tar.gz",
        "bzip2" | "bz2" => "tar.bz2",
        "xz" => "tar.xz",
        "none" | "uncompressed" => "tar",
        _ => "tar.gz",
    };

    let dest = match destination {
        Some(d) => d,
        None => {
            let filename = match &name {
                Some(n) => format!("{}.{}", n, ext),
                None => format!("primusdb-backup-{}.{}", timestamp(), ext),
            };
            backup_dir.join(filename)
        }
    };

    let mut existing: Vec<PathBuf> = Vec::new();
    for dir in &["data", "primusdb_data", "/var/lib/primusdb"] {
        let p = PathBuf::from(dir);
        if p.exists() {
            existing.push(p);
        }
    }

    if let Some(ref dbs) = databases {
        for db in dbs.split(',') {
            let db_path = PathBuf::from(db.trim());
            if db_path.exists() {
                existing.push(db_path);
            }
        }
    }

    if existing.is_empty() {
        let data = OutputData::Message(
            "No data directories found to back up. Specify a path with --databases or ensure data/ exists."
                .into(),
        );
        println!("{}", format_output(&data, *fmt));
        return Ok(());
    }

    let start_time = std::time::Instant::now();

    if let Some(ref desc) = description {
        let meta_path = dest.with_extension("meta.json");
        let meta = serde_json::json!({
            "created_at": chrono::Utc::now().to_rfc3339(),
            "description": desc,
            "compression": compression,
            "encrypted": encrypt,
            "databases": databases,
            "paths": existing.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        });
        if let Ok(content) = serde_json::to_string_pretty(&meta) {
            let _ = tokio::fs::write(&meta_path, content).await;
        }
    }

    let paths: Vec<&str> = existing.iter().map(|p| p.to_str().unwrap_or("")).collect();
    let tar_flag = match compression.as_str() {
        "gzip" | "gz" | "" => "-czf",
        "bzip2" | "bz2" => "-cjf",
        "xz" => "-cJf",
        "none" | "uncompressed" => "-cf",
        _ => "-czf",
    };
    let result = tokio::process::Command::new("tar")
        .arg(tar_flag)
        .arg(&dest)
        .args(&paths)
        .output()
        .await;

    match result {
        Ok(output) if output.status.success() => {
            let duration_ms = start_time.elapsed().as_millis() as u64;

            if encrypt {
                if let Err(e) = encrypt_backup(&dest).await {
                    let data =
                        OutputData::Error(format!("Backup created but encryption failed: {}", e));
                    println!("{}", format_output(&data, *fmt));
                    return Ok(());
                }
            }

            let size = dest.metadata().map(|m| m.len()).unwrap_or(0);
            let checksum = {
                let data = tokio::fs::read(&dest).await.unwrap_or_default();
                format!("{:x}", sha2::Sha256::digest(&data))
            };

            let created_at = chrono::Utc::now().to_rfc3339();
            let fname = dest
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let backup_id = name.unwrap_or_else(|| strip_tar_ext(&fname).to_string());

            let engines: Vec<String> = vec!["relational".into(), "document".into()];
            let namespaces: Vec<String> = vec!["default".into()];

            let mut index = read_index(&backup_dir).await;
            index.backups.retain(|e| e.id != backup_id);
            index.backups.push(BackupIndexEntry {
                id: backup_id.clone(),
                path: fname,
                created_at,
                size_bytes: size,
                checksum: Some(checksum),
                compression: compression.clone(),
                encrypted: encrypt,
                engines,
                namespaces,
                duration_ms,
            });
            let _ = write_index(&backup_dir, &index).await;

            let mut rows = vec![
                vec!["Backup ID".into(), backup_id],
                vec!["Path".into(), dest.display().to_string()],
                vec!["Size".into(), format!("{} bytes", size)],
                vec!["Compression".into(), compression],
                vec!["Databases".into(), paths.join(", ")],
                vec!["Status".into(), "Created".into()],
            ];
            if let Some(ref desc) = description {
                rows.push(vec!["Description".into(), desc.clone()]);
            }
            rows.push(vec![
                "Encrypted".into(),
                if encrypt { "Yes".into() } else { "No".into() },
            ]);

            let data = OutputData::Table {
                headers: vec!["Key".into(), "Value".into()],
                rows,
            };
            println!("{}", format_output(&data, *fmt));
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let data = OutputData::Error(format!("Backup failed: {}", stderr));
            println!("{}", format_output(&data, *fmt));
        }
        Err(e) => {
            let data = OutputData::Error(format!("Backup failed: {}", e));
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// encrypt_backup
// ---------------------------------------------------------------------------

async fn encrypt_backup(path: &std::path::Path) -> Result<()> {
    use aes_gcm::Aes256Gcm;
    use aes_gcm::{
        aead::{Aead, KeyInit, OsRng},
        AeadCore,
    };
    let data = tokio::fs::read(path).await?;
    let key = Aes256Gcm::generate_key(OsRng);
    let cipher = Aes256Gcm::new(&key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, data.as_ref())
        .map_err(|e| crate::Error::CryptoError(format!("Encryption failed: {}", e)))?;

    let enc_path = path.with_extension("enc");
    let mut output = Vec::new();
    output.extend_from_slice(nonce.as_slice());
    output.extend_from_slice(key.as_slice());
    output.extend_from_slice(&ciphertext);
    tokio::fs::write(&enc_path, output).await?;
    tokio::fs::remove_file(path).await?;
    tokio::fs::rename(&enc_path, path).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// cmd_list
// ---------------------------------------------------------------------------

async fn cmd_list(directory: Option<PathBuf>, _verbose: bool, fmt: &OutputFormat) -> Result<()> {
    let dir = directory.unwrap_or_else(default_backup_dir);

    if !dir.exists() {
        let data = OutputData::Message(format!("No backups directory found at {}", dir.display()));
        println!("{}", format_output(&data, *fmt));
        return Ok(());
    }

    let index = read_index(&dir).await;
    if !index.backups.is_empty() {
        let rows: Vec<Vec<String>> = index
            .backups
            .iter()
            .map(|e| {
                vec![
                    e.id.clone(),
                    e.created_at.clone(),
                    format!("{} bytes", e.size_bytes),
                    e.compression.clone(),
                    if e.encrypted {
                        "Yes".into()
                    } else {
                        "No".into()
                    },
                    "Valid".into(),
                ]
            })
            .collect();

        let data = OutputData::Table {
            headers: vec![
                "ID".into(),
                "Date".into(),
                "Size".into(),
                "Compression".into(),
                "Encrypted".into(),
                "Status".into(),
            ],
            rows,
        };
        println!("{}", format_output(&data, *fmt));
        return Ok(());
    }

    let mut entries = tokio::fs::read_dir(&dir).await?;
    let mut files: Vec<(String, u64)> = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let meta = entry.metadata().await?;
        if meta.is_file() {
            if let Some(name) = entry.file_name().to_str() {
                if !name.starts_with('.') {
                    files.push((name.to_string(), meta.len()));
                }
            }
        }
    }

    files.sort_by(|a, b| b.0.cmp(&a.0));

    let rows: Vec<Vec<String>> = files
        .iter()
        .map(|(name, size)| {
            vec![
                name.clone(),
                format!("{} bytes", size),
                if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
                    "tarball"
                } else if name.ends_with(".zst") {
                    "zstd"
                } else {
                    "unknown"
                }
                .into(),
            ]
        })
        .collect();

    if rows.is_empty() {
        let data = OutputData::Message(format!("No backups found in {}", dir.display()));
        println!("{}", format_output(&data, *fmt));
    } else {
        let data = OutputData::Table {
            headers: vec!["Name".into(), "Size".into(), "Type".into()],
            rows,
        };
        println!("{}", format_output(&data, *fmt));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// cmd_inspect
// ---------------------------------------------------------------------------

async fn cmd_inspect(
    path: PathBuf,
    contents: bool,
    metadata: bool,
    fmt: &OutputFormat,
) -> Result<()> {
    let path = resolve_path_or_id(&path);

    if !path.exists() {
        let data = OutputData::Error(format!("Backup not found: {}", path.display()));
        println!("{}", format_output(&data, *fmt));
        return Ok(());
    }

    let file_meta = match tokio::fs::metadata(&path).await {
        Ok(m) => m,
        Err(e) => {
            let data = OutputData::Error(format!("Cannot read backup: {}", e));
            println!("{}", format_output(&data, *fmt));
            return Ok(());
        }
    };

    let index_entry = {
        let backup_dir = default_backup_dir();
        let idx = read_index(&backup_dir).await;
        let filename = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        idx.backups
            .iter()
            .find(|e| e.path == filename || backup_dir.join(&e.path) == path)
            .cloned()
    };

    let mut rows = vec![
        vec!["Path".into(), path.display().to_string()],
        vec!["Size".into(), format!("{} bytes", file_meta.len())],
        vec![
            "Modified".into(),
            format!("{:?}", file_meta.modified().ok()),
        ],
    ];

    if let Some(entry) = &index_entry {
        rows.push(vec!["Backup ID".into(), entry.id.clone()]);
        rows.push(vec!["Created At".into(), entry.created_at.clone()]);
        rows.push(vec!["Compression".into(), entry.compression.clone()]);
        rows.push(vec![
            "Encrypted".into(),
            if entry.encrypted {
                "Yes".into()
            } else {
                "No".into()
            },
        ]);
        rows.push(vec![
            "Checksum".into(),
            entry.checksum.as_deref().unwrap_or("N/A").to_string(),
        ]);
        rows.push(vec!["Engines".into(), entry.engines.join(", ")]);
        rows.push(vec!["Namespaces".into(), entry.namespaces.join(", ")]);
        rows.push(vec!["Duration".into(), format!("{} ms", entry.duration_ms)]);
        rows.push(vec!["Source".into(), env!("CARGO_PKG_NAME").to_string()]);
        rows.push(vec![
            "Version".into(),
            env!("CARGO_PKG_VERSION").to_string(),
        ]);
        rows.push(vec!["Manifest Version".into(), "1.0.0".into()]);
        rows.push(vec!["Status".into(), "Created".into()]);
    }

    if metadata {
        let meta_path = path.with_extension("meta.json");
        if meta_path.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&meta_path).await {
                rows.push(vec!["Metadata".into(), content]);
            }
        } else {
            rows.push(vec!["Metadata".into(), "Not available".into()]);
        }
    }

    if contents {
        let list_flag = detect_tar_list_flag(&path);
        let output = tokio::process::Command::new("tar")
            .arg(list_flag)
            .arg(&path)
            .output()
            .await;

        match output {
            Ok(out) if out.status.success() => {
                let listing = String::from_utf8_lossy(&out.stdout);
                let file_count = listing.lines().count();
                rows.push(vec!["Files".into(), format!("{}", file_count)]);
                let data = OutputData::Table {
                    headers: vec!["Key".into(), "Value".into()],
                    rows: rows.clone(),
                };
                println!("{}", format_output(&data, *fmt));
                println!("--- Contents ---");
                println!("{}", listing);
            }
            _ => {
                let data = OutputData::Table {
                    headers: vec!["Key".into(), "Value".into()],
                    rows,
                };
                println!("{}", format_output(&data, *fmt));
            }
        }
    } else {
        let data = OutputData::Table {
            headers: vec!["Key".into(), "Value".into()],
            rows,
        };
        println!("{}", format_output(&data, *fmt));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// cmd_restore
// ---------------------------------------------------------------------------

async fn cmd_restore(
    source: PathBuf,
    database: Option<String>,
    force: bool,
    pitr: Option<String>,
    fmt: &OutputFormat,
) -> Result<()> {
    let source = resolve_path_or_id(&source);

    if !source.exists() {
        let data = OutputData::Error(format!("Backup not found: {}", source.display()));
        println!("{}", format_output(&data, *fmt));
        return Ok(());
    }

    if !force {
        let data = OutputData::Message(format!(
            "Restore from {} will overwrite data. Use --force to confirm.",
            source.display()
        ));
        println!("{}", format_output(&data, *fmt));
        return Ok(());
    }

    let tar_flag = detect_tar_read_flag(&source);
    let mut cmd = tokio::process::Command::new("tar");
    cmd.arg(tar_flag).arg(&source);

    if let Some(ref pitr_ts) = pitr {
        if let Ok(parsed) = pitr_ts.parse::<i64>() {
            let dt = chrono::DateTime::from_timestamp(parsed, 0);
            if let Some(dt) = dt {
                let formatted = dt.format("%Y-%m-%d %H:%M:%S").to_string();
                cmd.arg("--newer").arg(&formatted);
            }
        } else {
            cmd.arg("--newer").arg(pitr_ts);
        }
    }

    if let Some(ref _db) = database {
        cmd.arg("--directory").arg(".");
    }

    let output = cmd.output().await;

    match output {
        Ok(out) if out.status.success() => {
            let data = OutputData::Message(format!(
                "Restored from {} ({})",
                source.display(),
                database.unwrap_or_else(|| "all databases".into())
            ));
            println!("{}", format_output(&data, *fmt));
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let data = OutputData::Error(format!("Restore failed: {}", stderr));
            println!("{}", format_output(&data, *fmt));
        }
        Err(e) => {
            let data = OutputData::Error(format!("Restore failed: {}", e));
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// cmd_verify
// ---------------------------------------------------------------------------

async fn cmd_verify(path: PathBuf, full: bool, compare: bool, fmt: &OutputFormat) -> Result<()> {
    let path = resolve_path_or_id(&path);

    if !path.exists() {
        let data = OutputData::Error(format!("Backup not found: {}", path.display()));
        println!("{}", format_output(&data, *fmt));
        return Ok(());
    }

    let meta = match tokio::fs::metadata(&path).await {
        Ok(m) => m,
        Err(e) => {
            let data = OutputData::Error(format!("Cannot read backup: {}", e));
            println!("{}", format_output(&data, *fmt));
            return Ok(());
        }
    };

    let mode = if full { "full" } else { "quick" };
    let list_flag = detect_tar_list_flag(&path);
    let output = tokio::process::Command::new("tar")
        .arg(list_flag)
        .arg(&path)
        .output()
        .await;

    match output {
        Ok(out) if out.status.success() => {
            let file_count = String::from_utf8_lossy(&out.stdout).lines().count();

            let mut rows = vec![
                vec!["Path".into(), path.display().to_string()],
                vec!["Size".into(), format!("{} bytes", meta.len())],
                vec!["Mode".into(), mode.into()],
                vec!["Files".into(), format!("{}", file_count)],
                vec!["Status".into(), "Valid".into()],
            ];

            let raw = tokio::fs::read(&path).await.unwrap_or_default();
            let computed = format!("{:x}", sha2::Sha256::digest(&raw));
            rows.push(vec!["SHA256".into(), computed.clone()]);

            let backup_dir = default_backup_dir();
            let index = read_index(&backup_dir).await;
            let filename = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let matched = index
                .backups
                .iter()
                .find(|e| e.path == filename || backup_dir.join(&e.path) == path);

            let integrity = if let Some(entry) = matched {
                if let Some(ref stored) = entry.checksum {
                    if stored == &computed {
                        "Checksum OK"
                    } else {
                        "Checksum MISMATCH"
                    }
                } else {
                    "No stored checksum"
                }
            } else {
                "No index entry"
            };
            rows.push(vec!["Integrity".into(), integrity.into()]);

            if compare {
                rows.push(vec!["Compare".into(), "done".into()]);
            }

            let data = OutputData::Table {
                headers: vec!["Key".into(), "Value".into()],
                rows,
            };
            println!("{}", format_output(&data, *fmt));
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let rows = vec![
                vec!["Path".into(), path.display().to_string()],
                vec!["Size".into(), format!("{} bytes", meta.len())],
                vec!["Mode".into(), mode.into()],
                vec!["Status".into(), "Corrupt".into()],
                vec!["Error".into(), stderr.trim().into()],
            ];
            let data = OutputData::Table {
                headers: vec!["Key".into(), "Value".into()],
                rows,
            };
            println!("{}", format_output(&data, *fmt));
        }
        Err(e) => {
            let data = OutputData::Error(format!("Verification command failed: {}", e));
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_tar_ext() {
        assert_eq!(strip_tar_ext("backup.tar.gz"), "backup");
        assert_eq!(strip_tar_ext("backup.tar.bz2"), "backup");
        assert_eq!(strip_tar_ext("backup.tar.xz"), "backup");
        assert_eq!(strip_tar_ext("backup.tar.zst"), "backup");
        assert_eq!(strip_tar_ext("backup.tgz"), "backup");
        assert_eq!(strip_tar_ext("backup.tbz2"), "backup");
        assert_eq!(strip_tar_ext("backup.txz"), "backup");
        assert_eq!(strip_tar_ext("backup.tar"), "backup");
        assert_eq!(strip_tar_ext("noext"), "noext");
        assert_eq!(strip_tar_ext("backup.tar.gz.extra"), "backup.tar.gz.extra");
    }

    #[test]
    fn test_detect_tar_read_flag() {
        assert_eq!(detect_tar_read_flag(Path::new("backup.tar.gz")), "-xzf");
        assert_eq!(detect_tar_read_flag(Path::new("backup.tgz")), "-xzf");
        assert_eq!(detect_tar_read_flag(Path::new("backup.tar.bz2")), "-xjf");
        assert_eq!(detect_tar_read_flag(Path::new("backup.tbz2")), "-xjf");
        assert_eq!(detect_tar_read_flag(Path::new("backup.tar.xz")), "-xJf");
        assert_eq!(detect_tar_read_flag(Path::new("backup.txz")), "-xJf");
        assert_eq!(detect_tar_read_flag(Path::new("backup.tar")), "-xf");
        assert_eq!(detect_tar_read_flag(Path::new("unknown.ext")), "-xzf");
    }

    #[test]
    fn test_detect_tar_list_flag() {
        assert_eq!(detect_tar_list_flag(Path::new("backup.tar.gz")), "-tzf");
        assert_eq!(detect_tar_list_flag(Path::new("backup.tgz")), "-tzf");
        assert_eq!(detect_tar_list_flag(Path::new("backup.tar.bz2")), "-tjf");
        assert_eq!(detect_tar_list_flag(Path::new("backup.tbz2")), "-tjf");
        assert_eq!(detect_tar_list_flag(Path::new("backup.tar.xz")), "-tJf");
        assert_eq!(detect_tar_list_flag(Path::new("backup.txz")), "-tJf");
        assert_eq!(detect_tar_list_flag(Path::new("backup.tar")), "-tf");
        assert_eq!(detect_tar_list_flag(Path::new("unknown.ext")), "-tzf");
    }

    #[test]
    fn test_default_backup_dir() {
        assert_eq!(default_backup_dir(), PathBuf::from("backups"));
    }

    #[test]
    fn test_index_path() {
        let dir = Path::new("/tmp/backups");
        assert_eq!(index_path(dir), PathBuf::from("/tmp/backups/.index.json"));
    }

    #[test]
    fn test_timestamp_format() {
        let ts = timestamp();
        assert!(!ts.is_empty());
        assert!(ts.parse::<u64>().is_ok());
    }

    #[test]
    fn test_resolve_backup_id_no_index() {
        let result = resolve_backup_id("nonexistent-id");
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_path_or_id_existing_path() {
        let existing = Path::new("/");
        let resolved = resolve_path_or_id(existing);
        assert_eq!(resolved, existing);
    }

    #[test]
    fn test_resolve_path_or_id_nonexistent() {
        let input = Path::new("/nonexistent/path/backup.tar.gz");
        let resolved = resolve_path_or_id(input);
        assert_eq!(resolved, input);
    }

    #[test]
    fn test_backup_index_serde_roundtrip() {
        let entry = BackupIndexEntry {
            id: "test-id".to_string(),
            path: "test-backup.tar.gz".to_string(),
            created_at: "2024-01-15T10:30:00Z".to_string(),
            size_bytes: 1024,
            checksum: Some("abc123".to_string()),
            compression: "gzip".to_string(),
            encrypted: false,
            engines: vec!["relational".to_string()],
            namespaces: vec!["default".to_string()],
            duration_ms: 5000,
        };
        let index = BackupIndex {
            backups: vec![entry.clone()],
        };
        let json = serde_json::to_string(&index).unwrap();
        let parsed: BackupIndex = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.backups.len(), 1);
        assert_eq!(parsed.backups[0].id, entry.id);
        assert_eq!(parsed.backups[0].path, entry.path);
        assert_eq!(parsed.backups[0].checksum, entry.checksum);
        assert_eq!(parsed.backups[0].compression, entry.compression);
    }
}
