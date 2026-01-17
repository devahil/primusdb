# PrimusDB Troubleshooting Guide

This guide provides solutions for common issues encountered when running PrimusDB, including diagnostics, debugging techniques, and recovery procedures.

## Diagnostic Tools

### Health Check Commands
```bash
# Basic health check
curl http://localhost:8080/health

# Detailed status
curl http://localhost:8080/status

# Metrics endpoint
curl http://localhost:8080/metrics

# CLI status check
primusdb-cli status
```

### Log Analysis
```bash
# View recent logs
tail -f /var/log/primusdb/primusdb.log

# Search for errors
grep "ERROR" /var/log/primusdb/primusdb.log

# Filter by time
grep "2024-01-10" /var/log/primusdb/primusdb.log

# Count error types
grep "ERROR" /var/log/primusdb/primusdb.log | cut -d' ' -f4 | sort | uniq -c
```

### System Resource Monitoring
```bash
# CPU and memory usage
top -p $(pgrep primusdb)

# Disk I/O statistics
iostat -x 1

# Network connections
netstat -tlnp | grep :8080

# Open file descriptors
lsof -p $(pgrep primusdb) | wc -l
```

## Common Issues and Solutions

### 1. Server Won't Start

#### Symptoms
- Server fails to bind to port
- "Address already in use" error
- Permission denied errors

#### Diagnosis
```bash
# Check if port is in use
netstat -tlnp | grep :8080

# Check permissions
ls -la /var/lib/primusdb
ls -la /var/log/primusdb

# Check configuration
cat /etc/primusdb/primusdb.toml
```

#### Solutions
```bash
# Kill process using the port
sudo fuser -k 8080/tcp

# Fix permissions
sudo chown -R primusdb:primusdb /var/lib/primusdb
sudo chown -R primusdb:primusdb /var/log/primusdb

# Check configuration syntax
primusdb-server --config /etc/primusdb/primusdb.toml --dry-run
```

### 2. High Memory Usage

#### Symptoms
- Server consuming excessive RAM
- Out of memory errors
- Slow query performance

#### Diagnosis
```bash
# Check memory usage
ps aux | grep primusdb
free -h

# Check cache configuration
curl http://localhost:8080/metrics | grep cache

# Analyze heap usage (if debug build)
valgrind --tool=massif primusdb-server
```

#### Solutions
```toml
# Reduce cache size in config
[storage]
cache_size = 536870912  # 512MB instead of 1GB

# Enable memory limits
[limits]
max_memory_mb = 2048
memory_check_interval_seconds = 60
```

```bash
# Restart with reduced memory
systemctl restart primusdb

# Clear system cache
echo 3 > /proc/sys/vm/drop_caches
```

### 3. Slow Query Performance

#### Symptoms
- Queries taking longer than expected
- High CPU usage during queries
- Timeout errors

#### Diagnosis
```bash
# Enable query logging
export RUST_LOG=debug
primusdb-server --log-level debug

# Check query execution stats
curl http://localhost:8080/metrics | grep query_duration

# Analyze slow queries
grep "slow query" /var/log/primusdb/primusdb.log

# Check index usage
curl "http://localhost:8080/api/v1/table/columnar/sales/info"
```

#### Solutions
```bash
# Add indexes for frequently queried fields
# (Index creation via API - not implemented yet)
# For now, optimize query patterns

# Adjust query timeouts
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"timeout_ms": 30000}'

# Use pagination for large result sets
curl "http://localhost:8080/api/v1/crud/columnar/sales?limit=100&offset=0"
```

### 4. Connection Issues

#### Symptoms
- Client cannot connect to server
- "Connection refused" errors
- Network timeout errors

#### Diagnosis
```bash
# Test basic connectivity
ping localhost

# Check server status
curl -v http://localhost:8080/health

# Verify firewall rules
sudo ufw status
sudo iptables -L

# Check network configuration
ip addr show
netstat -tlnp | grep LISTEN
```

#### Solutions
```bash
# Open firewall port
sudo ufw allow 8080/tcp

# Check server binding
primusdb-server --host 0.0.0.0 --port 8080

# Verify configuration
cat /etc/primusdb/primusdb.toml | grep -A 5 network

# Test with different client
telnet localhost 8080
```

### 5. Data Corruption Issues

#### Symptoms
- Inconsistent query results
- "Data corruption detected" errors
- Failed integrity checks

#### Diagnosis
```bash
# Check data integrity
curl http://localhost:8080/api/v1/cache/cluster/health

# Verify checksums
primusdb-cli advanced analyze --storage-type columnar --table corrupted_table

# Check disk health
sudo smartctl -a /dev/sda
df -h

# Review recent operations
grep "ERROR\|WARN" /var/log/primusdb/primusdb.log | tail -20
```

#### Solutions
```bash
# Run consistency check
primusdb-cli advanced analyze --storage-type columnar --table problematic_table

# Repair corrupted data (if possible)
# Backup current data first
primusdb-cli backup --destination /backup/pre_repair_$(date +%Y%m%d)

# Restore from backup if corruption is severe
primusdb-cli restore --source /backup/latest_backup
```

### 6. Cluster Synchronization Problems

#### Symptoms
- Nodes out of sync
- Replication lag
- Inconsistent reads across nodes

#### Diagnosis
```bash
# Check cluster status
curl http://localhost:8080/api/v1/cluster/status

# Verify node synchronization
curl http://localhost:8080/api/v1/cache/cluster/health

# Check replication lag
for node in node1 node2 node3; do
  echo "Node $node:"
  curl -s http://$node:8080/status | jq .data.cluster
done
```

#### Solutions
```bash
# Force synchronization
curl -X POST http://coordinator:8080/api/v1/cluster/sync

# Rebalance shards
curl -X POST http://coordinator:8080/api/v1/cluster/rebalance

# Check and repair replication
primusdb-cli cluster repair --node problematic_node
```

### 7. AI/ML Operation Failures

#### Symptoms
- Prediction requests failing
- Model training errors
- Invalid prediction results

#### Diagnosis
```bash
# Check AI service status
curl http://localhost:8080/status | jq .data.ai_enabled

# Verify model existence
curl "http://localhost:8080/api/v1/models"

# Check prediction logs
grep "prediction\|model" /var/log/primusdb/primusdb.log

# Validate input data format
curl -X POST http://localhost:8080/api/v1/advanced/predict/columnar/sales \
  -H "Content-Type: application/json" \
  -d '{"model_id": "test", "input_data": {"invalid": "data"}}'
```

#### Solutions
```bash
# Retrain model with correct data
curl -X POST http://localhost:8080/api/v1/advanced/train \
  -H "Content-Type: application/json" \
  -d '{
    "table": "training_data",
    "model_type": "linear_regression",
    "target_column": "target",
    "feature_columns": ["feature1", "feature2"]
  }'

# Update model parameters
curl -X PUT http://localhost:8080/api/v1/models/model_id \
  -H "Content-Type: application/json" \
  -d '{"hyperparameters": {"learning_rate": 0.01}}'
```

## Performance Tuning

### Memory Optimization
```toml
# Optimize memory usage
[storage]
cache_size = 1073741824  # 1GB
compression = "lz4"

[limits]
max_memory_mb = 4096
gc_interval_seconds = 300
```

### CPU Optimization
```bash
# Set CPU affinity
taskset -c 0-7 primusdb-server

# Adjust thread pool size
export RAYON_NUM_THREADS=8
```

### I/O Optimization
```bash
# Use faster storage
# Move data directory to SSD
mv /var/lib/primusdb /mnt/ssd/primusdb
ln -s /mnt/ssd/primusdb /var/lib/primusdb

# Adjust I/O scheduler
echo "deadline" > /sys/block/sda/queue/scheduler
```

## Emergency Procedures

### 1. Service Restart
```bash
# Graceful restart
systemctl reload primusdb

# Force restart
systemctl restart primusdb

# Emergency stop
kill -9 $(pgrep primusdb)
```

### 2. Data Recovery
```bash
# Create emergency backup
primusdb-cli backup --destination /emergency_backup_$(date +%s)

# Restore from backup
primusdb-cli restore --source /path/to/good/backup

# Verify data integrity
primusdb-cli status
```

### 3. Cluster Recovery
```bash
# Isolate failing node
curl -X DELETE http://coordinator:8080/api/v1/cluster/nodes/failing_node

# Promote replica
curl -X POST http://coordinator:8080/api/v1/cluster/failover

# Rejoin node after repair
curl -X POST http://coordinator:8080/api/v1/cluster/nodes \
  -H "Content-Type: application/json" \
  -d '{"node_id": "repaired_node", "address": "10.0.0.1:8080"}'
```

## Log Collection for Support

### Comprehensive Diagnostic Bundle
```bash
#!/bin/bash
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BUNDLE_DIR="/tmp/primusdb_diagnostics_$TIMESTAMP"

mkdir -p "$BUNDLE_DIR"

# System information
uname -a > "$BUNDLE_DIR/system_info.txt"
free -h > "$BUNDLE_DIR/memory_info.txt"
df -h > "$BUNDLE_DIR/disk_info.txt"

# PrimusDB configuration
cp /etc/primusdb/primusdb.toml "$BUNDLE_DIR/" 2>/dev/null || echo "No config file"

# Recent logs
tail -1000 /var/log/primusdb/primusdb.log > "$BUNDLE_DIR/recent_logs.txt"

# Metrics snapshot
curl -s http://localhost:8080/metrics > "$BUNDLE_DIR/metrics.txt"

# Process information
ps aux | grep primusdb > "$BUNDLE_DIR/process_info.txt"
lsof -p $(pgrep primusdb) > "$BUNDLE_DIR/open_files.txt" 2>/dev/null

# Network information
netstat -tlnp > "$BUNDLE_DIR/network_info.txt"

# Create archive
tar -czf "/tmp/primusdb_diagnostics_$TIMESTAMP.tar.gz" -C /tmp "primusdb_diagnostics_$TIMESTAMP"

echo "Diagnostic bundle created: /tmp/primusdb_diagnostics_$TIMESTAMP.tar.gz"
```

### Support Information
When contacting support, please include:
1. **Diagnostic bundle** created above
2. **Steps to reproduce** the issue
3. **Expected vs actual behavior**
4. **Environment details** (OS, hardware, configuration)
5. **Timeline** of when the issue started
6. **Recent changes** to system or configuration

## Prevention Best Practices

### Regular Maintenance
```bash
# Daily health checks
#!/bin/bash
if ! curl -s http://localhost:8080/health | grep -q '"status":"healthy"'; then
    echo "PrimusDB health check failed" | mail -s "PrimusDB Alert" admin@example.com
fi

# Weekly backups
0 2 * * 1 primusdb-cli backup --destination /backup/weekly

# Monthly performance review
0 3 1 * * /path/to/performance_check.sh
```

### Monitoring Setup
```bash
# Install monitoring
sudo apt-get install prometheus grafana

# Configure alerts
# - Memory usage > 80%
# - Disk space < 10%
# - Query latency > 5 seconds
# - Error rate > 1%
# - Replication lag > 30 seconds
```

### Capacity Planning
- Monitor resource usage trends
- Plan for 30% headroom on all resources
- Implement auto-scaling where possible
- Regular performance benchmarking

This troubleshooting guide covers the most common issues and provides systematic approaches to diagnosis and resolution.