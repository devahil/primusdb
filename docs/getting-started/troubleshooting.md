# Troubleshooting

Common issues and their solutions when working with PrimusDB.

## Port Already in Use

**Error:** `Address already in use` or `Cannot bind to 0.0.0.0:8080`

**Cause:** Another process is already listening on the port.

**Solutions:**

```bash
# Find what is using the port
lsof -i :8080
# or
netstat -tlnp | grep :8080

# Kill the process (use the PID from above)
kill <PID>

# Or kill by port (Linux)
fuser -k 8080/tcp

# Or start PrimusDB on a different port
primusdb server start --bind 0.0.0.0:9090
```

## Permission Denied on Data Directory

**Error:** `Permission denied` when accessing the data directory or `Cannot create data directory`

**Cause:** The user running PrimusDB does not have write access to the configured `data_dir`.

**Solutions:**

```bash
# Check current ownership
ls -la ./data

# Fix ownership (replace 'primusdb' with your user)
sudo chown -R $(whoami) ./data

# Or use a directory you own
primusdb server start --data-dir /tmp/primusdb-data

# Or check the config file for a custom data_dir
cat primusdb.toml
grep data_dir primusdb.toml
```

## Server Won't Start

**Symptom:** `primusdb server start` exits immediately with no output or with an error.

**Diagnosis:**

```bash
# Increase log level to see what is happening
primusdb server start --log-level debug

# Verify the binary exists and is executable
ls -l target/release/primusdb
file target/release/primusdb

# Validate your configuration
primusdb config validate

# Run diagnostics
primusdb doctor

# Check system resources
free -h
df -h .
```

**Common causes:**

| Cause | Check | Fix |
|-------|-------|-----|
| Missing data directory | `ls ./data` | Create it or use `--data-dir` |
| Config file has syntax errors | `primusdb config validate` | Fix the TOML syntax |
| Port is privileged (< 1024) | `getcap target/release/primusdb` | Use a port >= 1024, or grant `CAP_NET_BIND_SERVICE` |
| Out of memory | `free -h` | Reduce `cache_size` in config |

## Connection Refused

**Error:** `Connection refused` when accessing `http://localhost:8080/health`

**Cause:** No PrimusDB server is listening on that address and port.

**Solutions:**

```bash
# Verify the server is running
primusdb server status

# Check if anything is listening on the port
lsof -i :8080

# If the server is running but on a different port
primusdb server start --bind 0.0.0.0:8080

# If the server is bound to a specific interface only (e.g., 127.0.0.1),
# connections from other machines will be refused. Check the config:
grep bind_address primusdb.toml

# For remote access, bind to 0.0.0.0
# [network]
# bind_address = "0.0.0.0"
```

## Build Failures

### Compiler Errors

```bash
# Make sure your Rust toolchain is up to date
rustup update stable
rustc --version   # Must be >= 1.70.0

# Clean and rebuild
cargo clean
cargo build --release
```

### Linker Errors

```bash
# Missing system dependencies
# Ubuntu/Debian
sudo apt-get install pkg-config libssl-dev build-essential

# Fedora
sudo dnf install openssl-devel

# Arch
sudo pacman -S base-devel openssl
```

### Out of Memory During Build

The release build is resource-intensive. Try:

```bash
# Reduce parallel jobs
CARGO_BUILD_JOBS=2 cargo build --release

# Or build in debug mode for testing
cargo build
```

## Missing Features

**Symptom:** A command or flag documented in the manual is not recognised.

**Possible causes:**

1. **Outdated binary.** You may be running an older build or the legacy binaries:

```bash
# Use the unified CLI
./target/release/primusdb --version

# The legacy binaries (primusdb-server, primusdb-cli) have fewer features
# and are deprecated. Always prefer `primusdb`.
```

2. **Not built from source.** There are no pre-compiled binaries. The version in your package manager (if any) may be outdated. Always build from the repository.

3. **Typo in command name.** Run the help command to see all available subcommands:

```bash
./target/release/primusdb --help
```

## How to Get Help

### Built-in Diagnostics

```bash
# Run the diagnostic tool
primusdb doctor

# Generate a verbose diagnostic report
primusdb doctor --aggressive --report diag.txt
```

### Checking the Logs

```bash
# If running in the foreground, check the terminal output
# If running as a daemon, look for logs
journalctl -u primusdb -f   # systemd
cat /var/log/primusdb/primusdb.log   # if log file is configured
```

### Gathering Information for Support

When reporting an issue, include:

- Output of `primusdb --version`
- Your OS and architecture (`uname -a` on Linux/macOS)
- The full error message (not just a summary)
- Steps to reproduce
- Your configuration file (`primusdb config show`)
- The diagnostic report (`primusdb doctor --report report.txt`)

### Filing an Issue

Report bugs and feature requests at:

[https://github.com/devahil/primusdb/issues](https://github.com/devahil/primusdb/issues)

Check the existing issues before filing a new one. The issue tracker includes a template (`ISSUES-1.3.1-alpha.md`) with guidelines for reporting problems.

### Community Resources

- **GitHub repository:** [https://github.com/devahil/primusdb](https://github.com/devahil/primusdb)
- **README:** Project overview, feature list, and quick start
- **Documentation:** Full documentation is available under the `docs/` directory in the repository
