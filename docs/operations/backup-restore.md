# Backup and Restore

This guide covers backup and restore operations for PrimusDB.

## `primusdb backup create`

Create a new backup of one or more databases.

```bash
# Create a full backup
primusdb backup create

# Backup specific databases
primusdb backup create --databases mydb,analytics

# Backup to a specific destination
primusdb backup create --destination /backups/daily/

# Create an encrypted backup
primusdb backup create --encrypt

# Backup with a description
primusdb backup create --description "Pre-upgrade backup"

# Choose compression algorithm
primusdb backup create --compression zstd
primusdb backup create --compression lz4
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `-d, --destination <PATH>` | Backup destination path | — |
| `-db, --databases <NAMES>` | Comma-separated list of databases | all |
| `--compression <ALGO>` | Compression algorithm (`zstd`, `lz4`, `none`) | `zstd` |
| `--encrypt` | Encrypt the backup | `false` |
| `-e, --description <TEXT>` | Backup description | — |

## `primusdb backup list`

List available backups.

```bash
# List backups in default location
primusdb backup list

# List backups in a specific directory
primusdb backup list --directory /backups/

# Show detailed backup information
primusdb backup list --directory /backups/ --verbose
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `-d, --directory <PATH>` | Backup directory to scan | — |
| `--verbose` | Show detailed backup info including size, date, databases | `false` |

## `primusdb backup inspect`

Inspect the contents and metadata of a backup archive.

```bash
# Inspect a backup
primusdb backup inspect /backups/primusdb-backup-20260115.zstd

# Show contents and metadata
primusdb backup inspect /backups/primusdb-backup-20260115.zstd --contents --metadata
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `--contents` | List the files and databases inside the backup | `false` |
| `--metadata` | Show backup metadata (timestamp, description, checksums) | `false` |

## `primusdb backup verify`

Verify the integrity of a backup archive.

```bash
# Quick verification
primusdb backup verify /backups/primusdb-backup-20260115.zstd

# Full integrity verification
primusdb backup verify /backups/primusdb-backup-20260115.zstd --full

# Compare checksums with stored metadata
primusdb backup verify /backups/primusdb-backup-20260115.zstd --compare
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `--full` | Perform full integrity verification (checks all data) | `false` |
| `--compare` | Compare backup checksums with stored metadata | `false` |

## `primusdb backup restore`

Restore a database from a backup archive.

```bash
# Full restore
primusdb backup restore /backups/primusdb-backup-20260115.zstd

# Restore a specific database
primusdb backup restore /backups/db-backup.zstd --database mydb

# Force restore (overwrite existing data)
primusdb backup restore /backups/backup.zstd --force

# Point-in-time recovery
primusdb backup restore /backups/backup.zstd --pitr "2026-01-15T12:00:00Z"
```

### Options

| Flag | Description |
|------|-------------|
| `-d, --database <NAME>` | Restore only a specific database from the backup |
| `--force` | Overwrite existing data without confirmation |
| `--pitr <TIMESTAMP>` | Point-in-time recovery timestamp (ISO 8601) |

## `primusdb restore`

Top-level convenience command for restoring a backup.

```bash
# Full restore
primusdb restore /backups/primusdb-backup-20260115.zstd

# Restore specific database
primusdb restore /backups/backup.zstd --database mydb --force
```

This is equivalent to `primusdb backup restore` and accepts the same flags.

## Backup Operations

### Encryption

Backups can be encrypted with AES-256-GCM using the `--encrypt` flag. The encryption key is generated at backup time and stored alongside the ciphertext in the backup file. Decryption is handled automatically during restore.

### Point-in-Time Recovery

Use `--pitr` with a Unix timestamp or date string to restore only files modified after that point:
```bash
primusdb backup restore backup.tar.gz --pitr 1719878400
primusdb backup restore backup.tar.gz --pitr "2024-07-01"
```

### Metadata

When a `--description` is provided, a `meta.json` file is created alongside the backup containing the description, timestamp, compression method, and encryption status.

## Manual Backup via Data Directory Copy

For environments where the CLI backup is not suitable, manual file-system-level backups can be used:

### Procedure

1. **Stop the server** (or ensure no writes are in progress):

```bash
primusdb server stop
```

2. **Copy the data directory**:

```bash
cp -a /var/lib/primusdb/data /backups/manual-20260115/
```

Or with compression:

```bash
tar -czf /backups/primusdb-data-20260115.tar.gz -C /var/lib/primusdb data
```

3. **Restart the server**:

```bash
primusdb server start
```

### Automated Script

```bash
#!/bin/bash
# manual-backup.sh — Create a file-level backup of PrimusDB data

BACKUP_DIR="${BACKUP_DIR:-/backups}"
DATE=$(date +%Y%m%d_%H%M%S)
DATA_DIR="${DATA_DIR:-/var/lib/primusdb/data}"
BACKUP_NAME="primusdb-manual-$DATE"

echo "Stopping PrimusDB..."
primusdb server stop --timeout 30

echo "Creating backup at $BACKUP_DIR/$BACKUP_NAME..."
cp -a "$DATA_DIR" "$BACKUP_DIR/$BACKUP_NAME"

echo "Compressing..."
tar -czf "$BACKUP_DIR/$BACKUP_NAME.tar.gz" -C "$BACKUP_DIR" "$BACKUP_NAME"
rm -rf "$BACKUP_DIR/$BACKUP_NAME"

echo "Starting PrimusDB..."
primusdb server start

echo "Backup complete: $BACKUP_DIR/$BACKUP_NAME.tar.gz"
```

### Restore from Manual Backup

```bash
# Stop the server
primusdb server stop

# Restore data directory
rm -rf /var/lib/primusdb/data
cp -a /backups/primusdb-manual-20260115 /var/lib/primusdb/data

# Or extract from archive
tar -xzf /backups/primusdb-data-20260115.tar.gz -C /var/lib/primusdb

# Start the server
primusdb server start
```

### Backup Strategy Recommendations

| Frequency | Type | Retention |
|-----------|------|-----------|
| Every 6 hours | Incremental file copy | 7 days |
| Daily | Full file copy + compress | 30 days |
| Weekly | Full file copy to remote storage | 90 days |
| Monthly | Full file copy to cold storage | 1 year |

### Remote Backup with rsync

```bash
# Sync to a remote server
rsync -avz --delete /var/lib/primusdb/data/ backup-server:/backups/primusdb/$(hostname)/

# Sync to S3-compatible storage
aws s3 sync /var/lib/primusdb/data/ s3://primusdb-backups/$(date +%Y/%m/%d)/
```
