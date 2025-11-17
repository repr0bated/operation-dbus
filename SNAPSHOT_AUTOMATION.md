# D-Bus Index Snapshot Automation

## Overview

The D-Bus index snapshot system uses a **rolling-3 retention policy**: it automatically keeps the 3 most recent snapshots and deletes older ones. This provides rollback capability without consuming excessive disk space.

## Automatic Snapshot Creation

### When Snapshots Are Created

Snapshots are **automatically created** in these scenarios:

1. **After index build**: `op-dbus index build`
   - Creates timestamped snapshot
   - Applies rolling-3 retention policy
   - Deletes snapshots older than the 3 most recent

2. **Manual snapshot**: `op-dbus index snapshot`
   - Create ad-hoc snapshot anytime
   - Can be tagged to prevent auto-deletion

3. **Scheduled updates**: Via cron or systemd timer (see below)

## Update Frequencies

### Option 1: Systemd Timer (Recommended)

**Frequency:** Daily at 2 AM (with 1-hour random delay)

```bash
# Install systemd units
sudo cp systemd/op-dbus-index.service /etc/systemd/system/
sudo cp systemd/op-dbus-index.timer /etc/systemd/system/

# Enable and start timer
sudo systemctl daemon-reload
sudo systemctl enable op-dbus-index.timer
sudo systemctl start op-dbus-index.timer

# Check timer status
systemctl status op-dbus-index.timer
systemctl list-timers op-dbus-index.timer

# View logs
journalctl -u op-dbus-index.service -f
```

**Customizing frequency:**

```bash
# Edit timer unit
sudo systemctl edit op-dbus-index.timer

# Examples:
# Every 6 hours:
OnCalendar=00/6:00

# Twice daily (6 AM and 6 PM):
OnCalendar=06:00
OnCalendar=18:00

# Weekly on Sunday at 3 AM:
OnCalendar=Sun 03:00

# Hourly:
OnCalendar=hourly
```

### Option 2: Cron Job

**Frequency:** Configurable via crontab

```bash
# Install script
sudo cp scripts/auto-snapshot-dbus-index.sh /usr/local/bin/
sudo chmod +x /usr/local/bin/auto-snapshot-dbus-index.sh

# Edit crontab
sudo crontab -e

# Daily at 2 AM:
0 2 * * * /usr/local/bin/auto-snapshot-dbus-index.sh

# Every 6 hours:
0 */6 * * * /usr/local/bin/auto-snapshot-dbus-index.sh

# Twice daily (6 AM and 6 PM):
0 6,18 * * * /usr/local/bin/auto-snapshot-dbus-index.sh

# Weekly on Sunday at 3 AM:
0 3 * * 0 /usr/local/bin/auto-snapshot-dbus-index.sh

# Hourly:
0 * * * * /usr/local/bin/auto-snapshot-dbus-index.sh
```

View cron logs:
```bash
tail -f /var/log/op-dbus/auto-index.log
```

### Option 3: Manual Updates

**Frequency:** On-demand

```bash
# Rebuild index (creates snapshot automatically)
op-dbus index build

# Create snapshot without rebuilding
op-dbus index snapshot

# Create tagged snapshot (won't be auto-deleted)
op-dbus index snapshot --tag golden-master-2025-11-16
```

### Option 4: Event-Driven

**Frequency:** After system changes

```bash
# In your deployment scripts
apt-get install new-package
systemctl daemon-reload
op-dbus index build  # Auto-snapshot

# Or via systemd path unit (watches for changes)
# /etc/systemd/system/op-dbus-index-trigger.path
```

## Recommended Frequencies by Use Case

### Development/Testing
```
Frequency: Hourly or after each significant change
Method: Manual or cron
Retention: Rolling-3 is sufficient
```

### Production Systems (Stable)
```
Frequency: Daily at 2 AM
Method: Systemd timer
Retention: Rolling-3 (keep last 3 days)
```

### Production Systems (Active Development)
```
Frequency: Every 6 hours
Method: Systemd timer or cron
Retention: Rolling-3 (covers last 18 hours)
```

### Golden Master Nodes
```
Frequency: Weekly or manual
Method: Manual with tags
Retention: Tagged snapshots kept forever
```

### CI/CD Pipeline
```
Frequency: After each deployment
Method: Event-driven (in deploy script)
Retention: Rolling-3
```

## Snapshot Management Commands

### List Snapshots
```bash
op-dbus index snapshots

# Output:
# === D-Bus Index Snapshots (3) ===
#
#   2025-11-16-020000 - 2025-11-16T02:00:00+00:00
#   2025-11-15-020000 - 2025-11-15T02:00:00+00:00
#   2025-11-14-020000 - 2025-11-14T02:00:00+00:00 [TAGGED: golden-master]
```

### Create Manual Snapshot
```bash
# Timestamped snapshot
op-dbus index snapshot

# Named snapshot
op-dbus index snapshot --name before-upgrade

# Tagged snapshot (won't be auto-deleted)
op-dbus index snapshot --tag production-v1.0
```

### Clean Up Old Snapshots
```bash
# Interactive cleanup
op-dbus index cleanup

# Force cleanup (no confirmation)
op-dbus index cleanup --force
```

### Compare Snapshots
```bash
op-dbus index diff \
    /var/lib/op-dbus/@snapshots/dbus-index/2025-11-15-020000 \
    /var/lib/op-dbus/@dbus-index

# Output shows services added/removed since snapshot
```

## Retention Policy Details

### Rolling-3 Policy

**How it works:**
1. After creating a new snapshot, count total snapshots
2. If count > 3, delete oldest untagged snapshots
3. Keep exactly 3 most recent untagged snapshots
4. Tagged snapshots are NEVER auto-deleted

**Example timeline:**
```
Day 1: Create snapshot-1                      → Snapshots: [1]
Day 2: Create snapshot-2                      → Snapshots: [1, 2]
Day 3: Create snapshot-3                      → Snapshots: [1, 2, 3]
Day 4: Create snapshot-4, delete snapshot-1   → Snapshots: [2, 3, 4]
Day 5: Create snapshot-5, delete snapshot-2   → Snapshots: [3, 4, 5]
```

**With tags:**
```
Day 1: Create snapshot-1, tag as "golden"     → Snapshots: [1*]
Day 2: Create snapshot-2                      → Snapshots: [1*, 2]
Day 3: Create snapshot-3                      → Snapshots: [1*, 2, 3]
Day 4: Create snapshot-4                      → Snapshots: [1*, 2, 3, 4]
Day 5: Create snapshot-5, delete snapshot-2   → Snapshots: [1*, 3, 4, 5]
                                                (snapshot-1 kept because tagged)
```

### Customizing Retention

```rust
// In code - change keep value
RetentionPolicy::Rolling { keep: 5 }  // Keep last 5

// Or use time-based retention
RetentionPolicy::TimeBased { days: 7 }  // Keep 7 days

// Or mix tagged + rolling
RetentionPolicy::Tagged { keep_untagged: 3 }  // Keep all tagged + last 3 untagged
```

## Disk Space Management

### Snapshot Size

- **Full index**: ~100 MB (compressed)
- **3 snapshots**: ~300 MB total
- **BTRFS deduplication**: Actual usage ~150-200 MB (shared data)

### Monitor Disk Usage

```bash
# Check snapshot directory size
du -sh /var/lib/op-dbus/@snapshots/dbus-index

# BTRFS-aware size (shows actual disk usage)
btrfs filesystem du /var/lib/op-dbus/@snapshots/dbus-index

# List snapshots with sizes
for snap in /var/lib/op-dbus/@snapshots/dbus-index/*; do
    echo "$(basename $snap): $(du -sh $snap | cut -f1)"
done
```

## Rollback Procedure

### Rollback to Previous Snapshot

```bash
# 1. List snapshots
op-dbus index snapshots

# 2. Choose snapshot to restore
SNAPSHOT="/var/lib/op-dbus/@snapshots/dbus-index/2025-11-15-020000"

# 3. Backup current index
mv /var/lib/op-dbus/@dbus-index /var/lib/op-dbus/@dbus-index.broken

# 4. Restore snapshot
btrfs subvolume snapshot $SNAPSHOT /var/lib/op-dbus/@dbus-index

# 5. Verify
op-dbus index stats

# 6. Delete broken index if successful
btrfs subvolume delete /var/lib/op-dbus/@dbus-index.broken
```

## Monitoring & Alerts

### Check Last Update Time

```bash
# Via stats
op-dbus index stats | grep "Last updated"

# Via file timestamp
stat -c '%y' /var/lib/op-dbus/@dbus-index/index.json

# Via systemd (if using timer)
systemctl status op-dbus-index.service
```

### Alert on Stale Index

```bash
#!/bin/bash
# Alert if index is older than 2 days

INDEX_TIME=$(stat -c '%Y' /var/lib/op-dbus/@dbus-index/index.json)
NOW=$(date +%s)
AGE_SECONDS=$((NOW - INDEX_TIME))
AGE_HOURS=$((AGE_SECONDS / 3600))

if [ $AGE_HOURS -gt 48 ]; then
    echo "⚠️  D-Bus index is ${AGE_HOURS} hours old (>48 hours)"
    # Send alert (email, webhook, etc.)
fi
```

### Integration with Monitoring Systems

```bash
# Prometheus node_exporter textfile
cat > /var/lib/node_exporter/textfile_collector/op_dbus.prom <<EOF
# HELP op_dbus_index_age_seconds Age of D-Bus index in seconds
# TYPE op_dbus_index_age_seconds gauge
op_dbus_index_age_seconds $AGE_SECONDS

# HELP op_dbus_snapshots_total Total number of D-Bus snapshots
# TYPE op_dbus_snapshots_total gauge
op_dbus_snapshots_total $(op-dbus index snapshots | grep "Total:" | awk '{print $2}')
EOF
```

## Best Practices

1. **Use systemd timer for production**
   - Built-in persistence and randomization
   - Better logging via journald
   - Resource control via systemd

2. **Tag important snapshots**
   - Before major upgrades
   - After successful deployments
   - Golden master configurations

3. **Monitor snapshot age**
   - Alert if index is stale
   - Track in monitoring system

4. **Test rollback procedure**
   - Practice restoring from snapshots
   - Document steps for team

5. **Keep rolling-3 for most use cases**
   - Balances rollback capability with disk usage
   - 3 snapshots = 3 rollback points
   - Adjust if needed (keep: 5 or keep: 7)

## Troubleshooting

### Snapshots Not Being Created

```bash
# Check if BTRFS is available
which btrfs

# Check if directory is a BTRFS volume
btrfs subvolume show /var/lib/op-dbus/@dbus-index

# Check permissions
ls -la /var/lib/op-dbus/@snapshots/

# Check logs
journalctl -u op-dbus-index.service -n 50
tail /var/log/op-dbus/auto-index.log
```

### Too Many Snapshots

```bash
# Force cleanup
op-dbus index cleanup --force

# Or manually delete old snapshots
btrfs subvolume delete /var/lib/op-dbus/@snapshots/dbus-index/OLD_SNAPSHOT_NAME
```

### Snapshot Failed During Index Build

```bash
# Index is still saved even if snapshot fails
# Manually create snapshot:
op-dbus index snapshot

# Check if subvolume exists:
btrfs subvolume list /var/lib/op-dbus
```

## Summary

**Default behavior:**
- ✅ Auto-snapshot after `op-dbus index build`
- ✅ Rolling-3 retention (keep last 3)
- ✅ BTRFS-native snapshots (fast, space-efficient)

**Recommended for most users:**
- **Frequency**: Daily via systemd timer
- **Retention**: Rolling-3 (default)
- **Tags**: Use for golden masters and pre-upgrade snapshots

**Quick start:**
```bash
# Enable automatic daily updates
sudo systemctl enable --now op-dbus-index.timer

# Check it's running
systemctl status op-dbus-index.timer
```
