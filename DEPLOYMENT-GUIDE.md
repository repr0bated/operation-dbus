# Operation-DBus Complete Deployment Guide
**Purpose:** Deploy all features from git to the live system
**Time Required:** 15-30 minutes
**Prerequisites:** Root access, Rust toolchain, BTRFS (optional)

---

## Quick Start (TL;DR)

If you just want to get everything running:

```bash
cd /home/user/operation-dbus

# Build binaries with new features
cargo build --release --bin dbus-mcp --features mcp

# Deploy using automated staging tool
sudo staging/op-dbus-stage-v2 interactive
# Select option 4: "Install all (recommended)"

# Start MCP server
sudo dbus-mcp --port 8080 --enable-web --web-root src/mcp/web &

# Access web UI
# Open browser to: http://localhost:8080
```

---

## Understanding What Will Be Deployed

This deployment will activate **4 major features** currently sitting in git:

### 1. D-Bus Discovery Tools (Commit c401069)
- Three new MCP tools for AI agents to explore D-Bus
- Makes the system self-documenting
- **Deployment:** Build `dbus-mcp` binary, start server

### 2. Discovery Web Interface (Commit 3ef654c)
- Tree-view UI for exploring D-Bus services
- Interactive controls, statistics dashboard
- **Deployment:** Start MCP server with `--enable-web`

### 3. Workflow Builder (Commit 81a9fc9)
- Visual programming interface like Node-RED
- Drag-and-drop automation workflows
- **Deployment:** Already in web files, served by MCP server

### 4. Staging Deployment System (Commit da7979f)
- BTRFS snapshot-based deployment
- Verification and rollback capabilities
- **Deployment:** Run the staging tool itself

---

## Deployment Methods

Choose one of three methods based on your preference:

### Method A: Automated (Recommended) ⭐
Uses the staging tool for managed deployment with snapshots.

### Method B: Manual
Step-by-step commands if you want full control.

### Method C: Quick & Dirty
Fastest way to just get it running (no staging, no snapshots).

---

## Method A: Automated Deployment (Recommended)

This uses the staging tool you created. It provides:
- ✅ BTRFS snapshots before/after each step
- ✅ Organized staging folders
- ✅ Automatic verification
- ✅ Easy rollback if needed
- ✅ System symlinks for actual integration

### Step 1: Initialize Staging Environment

```bash
cd /home/user/operation-dbus
sudo staging/op-dbus-stage-v2 init
```

**Expected Output:**
```
[INFO] Initializing staging environment at /var/lib/op-dbus/staging
[INFO] Creating BTRFS subvolume for staging...
[SUCCESS] Staging environment initialized
```

**What This Does:**
- Creates `/var/lib/op-dbus/staging` (as BTRFS subvolume if possible)
- Creates `/var/lib/op-dbus/staging/snapshots` directory
- Initializes state tracking file

### Step 2: Run Interactive Installation

```bash
sudo staging/op-dbus-stage-v2 interactive
```

**Expected Menu:**
```
╔═══════════════════════════════════════════════╗
║   Operation-DBus Staged Installation Tool    ║
╚═══════════════════════════════════════════════╝

Available components:

  1. core-dbus              - Core D-Bus infrastructure (required)
  2. introspection-cache    - SQLite cache for fast queries
  3. mcp-server             - AI agent integration + web UI

  4. Install all (recommended)
  0. Exit

Select component [1-4]:
```

**Select: 4** (Install all)

### Step 3: What Happens During Installation

The tool will automatically:

#### Stage 1: Deploy core-dbus
```
[INFO] Creating stage folder: stage-1-core-dbus
[INFO] Building core D-Bus binary...
   Compiling operation-dbus v0.1.0
    Finished release [optimized] target(s)
[INFO] Copying files to staging...
[INFO] Creating system symlinks...
[SUCCESS] Stage 1 (core-dbus) deployed

=== Verifying: core-dbus ===
✓ Binary symlink correct: /usr/local/bin/op-dbus → /var/lib/op-dbus/staging/stage-1-core-dbus/bin/op-dbus
✓ Binary is executable
[SUCCESS] Stage verification passed: core-dbus
```

**System Changes:**
- Binary built: `target/release/op-dbus`
- Staging folder: `/var/lib/op-dbus/staging/stage-1-core-dbus/`
- System symlink: `/usr/local/bin/op-dbus` → staging folder
- Snapshots: `pre-core-dbus-TIMESTAMP`, `post-core-dbus-TIMESTAMP`

#### Stage 2: Deploy introspection-cache
```
[INFO] Creating stage folder: stage-2-introspection-cache
[INFO] Copying introspection cache files...
[INFO] Creating system symlinks...
[INFO] Enabling systemd timer...
[SUCCESS] Stage 2 (introspection-cache) deployed

=== Verifying: introspection-cache ===
✓ Cache warmup script symlinked
✓ Timer enabled
✓ Timer running
[SUCCESS] Stage verification passed: introspection-cache
```

**System Changes:**
- Staging folder: `/var/lib/op-dbus/staging/stage-2-introspection-cache/`
- System symlinks:
  - `/usr/local/bin/warm-dbus-cache.sh` → staging folder
  - `/etc/systemd/system/dbus-cache-warmup.service` → staging folder
  - `/etc/systemd/system/dbus-cache-warmup.timer` → staging folder
- Systemd timer: `dbus-cache-warmup.timer` enabled and running
- Snapshots: `pre-introspection-cache-TIMESTAMP`, `post-introspection-cache-TIMESTAMP`

#### Stage 3: Deploy mcp-server
```
[INFO] Creating stage folder: stage-5-mcp-server
[INFO] Building MCP server...
   Compiling dbus-mcp v0.1.0
    Finished release [optimized] target(s)
[INFO] Copying files to staging...
[INFO] Creating system symlinks...
[SUCCESS] Stage 5 (mcp-server) deployed

=== Verifying: mcp-server ===
✓ MCP server binary symlinked
✓ Web UI symlinked
[SUCCESS] Stage verification passed: mcp-server

All components installed!

Access the web UI at: http://localhost:8080
Start MCP server: sudo dbus-mcp --port 8080 --enable-web
```

**System Changes:**
- Binary built: `target/release/dbus-mcp` (includes discovery tools!)
- Staging folder: `/var/lib/op-dbus/staging/stage-5-mcp-server/`
- System symlinks:
  - `/usr/local/bin/dbus-mcp` → staging folder
  - `/var/www/op-dbus-mcp` → staging folder web files
- Snapshots: `pre-mcp-server-TIMESTAMP`, `post-mcp-server-TIMESTAMP`

### Step 4: Start MCP Server

```bash
# Option A: Foreground (see logs)
sudo dbus-mcp --port 8080 --enable-web --web-root /var/www/op-dbus-mcp

# Option B: Background (daemon-like)
sudo dbus-mcp --port 8080 --enable-web --web-root /var/www/op-dbus-mcp > /var/log/dbus-mcp.log 2>&1 &

# Option C: Using systemd (create service first - see Appendix A)
sudo systemctl start dbus-mcp
```

**Expected Output:**
```
MCP Server starting...
Web UI enabled at http://0.0.0.0:8080
Serving files from: /var/www/op-dbus-mcp
Discovery tools loaded: list_dbus_services, list_dbus_object_paths, introspect_dbus_object
Listening on port 8080...
```

### Step 5: Verify Deployment

```bash
# Check staging status
staging/op-dbus-stage-v2 status

# Check MCP server is running
ps aux | grep dbus-mcp

# Check port is listening
ss -tlnp | grep 8080

# Test web UI (should return HTML)
curl -I http://localhost:8080
```

**Expected Status Output:**
```
=== Staging Status ===

Staging Base: /var/lib/op-dbus/staging

✓ stage-1-core-dbus [4.2M]
✓ stage-2-introspection-cache [1.1M]
✓ stage-5-mcp-server [52M]

=== Snapshots ===

● pre-core-dbus-20251118-100522 [512K] 20251118-100522
● post-core-dbus-20251118-100534 [512K] 20251118-100534
● pre-introspection-cache-20251118-100601 [512K] 20251118-100601
● post-introspection-cache-20251118-100615 [512K] 20251118-100615
● pre-mcp-server-20251118-100703 [512K] 20251118-100703
● post-mcp-server-20251118-100721 [512K] 20251118-100721

Total: 6 snapshots
```

### Step 6: Access Features

#### Web UI (Discovery + Workflow Builder)
Open browser to: **http://localhost:8080**

You should see:
- **Discovery Tab:** Tree view of D-Bus services, click "Discover Services"
- **Workflow Tab:** Drag-and-drop workflow builder

#### MCP Tools (For AI Agents)
The MCP server exposes these tools via JSON-RPC:
- `list_dbus_services` - Get all services
- `list_dbus_object_paths` - Get paths for a service
- `introspect_dbus_object` - Get details for an object

Test via curl:
```bash
curl -X POST http://localhost:8080/api/tools/list_dbus_services/execute \
  -H "Content-Type: application/json" \
  -d '{"include_activatable": false}'
```

### Success Criteria ✅

After Method A deployment, you should have:

- [ ] Staging directory exists: `/var/lib/op-dbus/staging`
- [ ] 3 stage folders created: `stage-1-core-dbus`, `stage-2-introspection-cache`, `stage-5-mcp-server`
- [ ] 6+ BTRFS snapshots in `staging/snapshots/`
- [ ] System symlinks: `/usr/local/bin/{op-dbus,dbus-mcp,warm-dbus-cache.sh}`
- [ ] Systemd timer running: `dbus-cache-warmup.timer`
- [ ] MCP server process running on port 8080
- [ ] Web UI accessible at http://localhost:8080
- [ ] Discovery tab shows D-Bus tree view
- [ ] Workflow tab shows node palette and canvas

---

## Method B: Manual Deployment

If you prefer step-by-step control without the staging tool:

### Step 1: Build Binaries

```bash
cd /home/user/operation-dbus

# Build core binary
cargo build --release --bin op-dbus

# Build MCP server (includes discovery tools)
cargo build --release --bin dbus-mcp --features mcp
```

**Time:** 5-15 minutes depending on system
**Output:** Binaries in `target/release/`

### Step 2: Install Core Binary

```bash
sudo cp target/release/op-dbus /usr/local/bin/
sudo chmod +x /usr/local/bin/op-dbus
```

### Step 3: Install MCP Server

```bash
sudo cp target/release/dbus-mcp /usr/local/bin/
sudo chmod +x /usr/local/bin/dbus-mcp
```

### Step 4: Install Web Files

```bash
sudo mkdir -p /var/www/op-dbus-mcp
sudo cp -r src/mcp/web/* /var/www/op-dbus-mcp/
```

### Step 5: Install Cache Warmup

```bash
sudo cp scripts/warm-dbus-cache.sh /usr/local/bin/
sudo chmod +x /usr/local/bin/warm-dbus-cache.sh

sudo cp systemd/dbus-cache-warmup.service /etc/systemd/system/
sudo cp systemd/dbus-cache-warmup.timer /etc/systemd/system/

sudo systemctl daemon-reload
sudo systemctl enable dbus-cache-warmup.timer
sudo systemctl start dbus-cache-warmup.timer
```

### Step 6: Start MCP Server

```bash
sudo dbus-mcp --port 8080 --enable-web --web-root /var/www/op-dbus-mcp &
```

### Step 7: Verify

```bash
# Check binaries
ls -lh /usr/local/bin/{op-dbus,dbus-mcp,warm-dbus-cache.sh}

# Check systemd
systemctl status dbus-cache-warmup.timer

# Check MCP server
curl -I http://localhost:8080
```

---

## Method C: Quick & Dirty

Fastest way to just see it working (no installation, run from repo):

```bash
cd /home/user/operation-dbus

# Build
cargo build --release --bin dbus-mcp --features mcp

# Run directly from target
sudo target/release/dbus-mcp --port 8080 --enable-web --web-root src/mcp/web
```

**Access:** http://localhost:8080

**Note:** This doesn't install anything, just runs from the repo. Server stops when you close terminal.

---

## Post-Deployment Testing

### Test 1: Discovery Tools (Backend)

```bash
# Test list services
curl -X POST http://localhost:8080/api/tools/list_dbus_services/execute \
  -H "Content-Type: application/json" \
  -d '{"include_activatable": false}' | jq .

# Test list paths
curl -X POST http://localhost:8080/api/tools/list_dbus_object_paths/execute \
  -H "Content-Type: application/json" \
  -d '{"service_name": "org.freedesktop.systemd1"}' | jq .

# Test introspection
curl -X POST http://localhost:8080/api/tools/introspect_dbus_object/execute \
  -H "Content-Type: application/json" \
  -d '{"service_name": "org.freedesktop.systemd1", "object_path": "/org/freedesktop/systemd1"}' | jq .
```

**Expected:** JSON responses with D-Bus data

### Test 2: Discovery Web UI

1. Open http://localhost:8080 in browser
2. Click "Discovery" tab
3. Click "Discover Services" button
4. **Expected:** List of D-Bus services appears (org.freedesktop.systemd1, etc.)
5. Click ">" next to a service to expand
6. **Expected:** Object paths appear under the service
7. Click ">" next to an object
8. **Expected:** Interfaces, methods, properties appear

### Test 3: Workflow Builder

1. Open http://localhost:8080 in browser
2. Click "Workflow" tab
3. Drag "D-Bus Method Call" node from palette to canvas
4. **Expected:** Node appears on canvas
5. Click the node
6. **Expected:** Properties panel shows on right with configuration fields
7. Try dragging more nodes and connecting them

### Test 4: Cache Warmup

```bash
# Check timer is running
systemctl status dbus-cache-warmup.timer

# Manually trigger warmup
sudo warm-dbus-cache.sh

# Check cache was created
ls -lh /var/cache/dbus-introspection.db
```

**Expected:** SQLite database file created/updated

---

## Troubleshooting

### Issue: "dbus-mcp: command not found"

**Cause:** Binary not in PATH or not built

**Fix:**
```bash
# Check if built
ls -lh target/release/dbus-mcp

# If missing, build it
cargo build --release --bin dbus-mcp --features mcp

# If built but not found, check PATH
which dbus-mcp
echo $PATH

# Install to system location
sudo cp target/release/dbus-mcp /usr/local/bin/
```

### Issue: "Port 8080 already in use"

**Cause:** Another process using port 8080

**Fix:**
```bash
# Find what's using the port
sudo ss -tlnp | grep 8080

# Use different port
sudo dbus-mcp --port 8081 --enable-web --web-root src/mcp/web
```

### Issue: "Web UI shows blank page"

**Cause:** Web files not found or incorrect path

**Fix:**
```bash
# Check web files exist
ls -lh src/mcp/web/index.html

# Check server was started with correct path
# Should be: --web-root src/mcp/web
# NOT: --web-root src/mcp/web/

# Restart with correct path
sudo dbus-mcp --port 8080 --enable-web --web-root $(pwd)/src/mcp/web
```

### Issue: "Discovery tools return errors"

**Cause:** Server built without new introspection.rs code

**Fix:**
```bash
# Check git commit
git log --oneline -1 src/mcp/tools/introspection.rs
# Should show commit c401069 or later

# Rebuild ensuring latest code
git pull
cargo clean
cargo build --release --bin dbus-mcp --features mcp
```

### Issue: "Systemd timer not starting"

**Cause:** Service file or script not found

**Fix:**
```bash
# Check files exist
ls -lh /etc/systemd/system/dbus-cache-warmup.{service,timer}
ls -lh /usr/local/bin/warm-dbus-cache.sh

# Check service status
systemctl status dbus-cache-warmup.service
systemctl status dbus-cache-warmup.timer

# Check logs
sudo journalctl -u dbus-cache-warmup.timer -n 50

# Reload and restart
sudo systemctl daemon-reload
sudo systemctl enable dbus-cache-warmup.timer
sudo systemctl start dbus-cache-warmup.timer
```

### Issue: "BTRFS snapshots failing"

**Cause:** Not on BTRFS filesystem

**Note:** This is OK! The staging tool will still work, it just won't create snapshots.

**Check filesystem:**
```bash
df -T /var/lib

# If not btrfs:
# The staging tool will use regular directories instead
# All functionality works except snapshots
```

**To still get snapshots on non-BTRFS:**
- Use LVM snapshots instead (manual)
- Or just skip snapshots (still get organized staging folders)

---

## Rollback Procedures

### Rollback Individual Component (Method A only)

```bash
# Rollback MCP server
sudo staging/op-dbus-stage-v2 rollback mcp-server

# This will:
# 1. Remove /usr/local/bin/dbus-mcp symlink
# 2. Remove /var/www/op-dbus-mcp symlink
# 3. Delete staging folder
# 4. Create rollback snapshot
```

### Rollback Entire Deployment (Method A)

```bash
# Rollback in reverse order
sudo staging/op-dbus-stage-v2 rollback mcp-server
sudo staging/op-dbus-stage-v2 rollback introspection-cache
sudo staging/op-dbus-stage-v2 rollback core-dbus

# Verify nothing deployed
staging/op-dbus-stage-v2 list
```

### Manual Cleanup (Method B or C)

```bash
# Stop services
sudo systemctl stop dbus-cache-warmup.timer
sudo pkill dbus-mcp

# Remove binaries
sudo rm /usr/local/bin/{op-dbus,dbus-mcp,warm-dbus-cache.sh}

# Remove systemd units
sudo rm /etc/systemd/system/dbus-cache-warmup.{service,timer}
sudo systemctl daemon-reload

# Remove web files
sudo rm -rf /var/www/op-dbus-mcp

# Remove staging (if using Method A)
sudo rm -rf /var/lib/op-dbus/staging
```

---

## Deployment Checklist

Use this to verify deployment was successful:

### Pre-Deployment
- [ ] On correct branch: `claude/ghostbfridge-non-nixos-improvements-018yb8CpSXQQuGGr8W1zz3TZ`
- [ ] All commits present (c401069, 3ef654c, 81a9fc9, da7979f)
- [ ] Rust toolchain available (`cargo --version` works)
- [ ] Root access available

### Build Phase
- [ ] `cargo build --release --bin op-dbus` succeeds
- [ ] `cargo build --release --bin dbus-mcp --features mcp` succeeds
- [ ] `target/release/op-dbus` exists (16MB+)
- [ ] `target/release/dbus-mcp` exists (16MB+)

### Deployment Phase (Method A)
- [ ] `sudo staging/op-dbus-stage-v2 init` succeeds
- [ ] Staging base created: `/var/lib/op-dbus/staging`
- [ ] Stage 1 deploys: `stage-1-core-dbus`
- [ ] Stage 2 deploys: `stage-2-introspection-cache`
- [ ] Stage 5 deploys: `stage-5-mcp-server`
- [ ] All verifications pass (green checkmarks)
- [ ] Snapshots created (6+ snapshots)

### System Integration
- [ ] Symlink exists: `/usr/local/bin/op-dbus`
- [ ] Symlink exists: `/usr/local/bin/dbus-mcp`
- [ ] Symlink exists: `/usr/local/bin/warm-dbus-cache.sh`
- [ ] Symlink exists: `/var/www/op-dbus-mcp`
- [ ] Systemd timer enabled: `dbus-cache-warmup.timer`
- [ ] Systemd timer active: `systemctl is-active dbus-cache-warmup.timer`

### Runtime
- [ ] MCP server starts without errors
- [ ] Port 8080 listening: `ss -tlnp | grep 8080`
- [ ] Process running: `ps aux | grep dbus-mcp`

### Web UI
- [ ] http://localhost:8080 accessible
- [ ] Page loads (no 404)
- [ ] Discovery tab visible
- [ ] Workflow tab visible
- [ ] "Discover Services" button works
- [ ] Services tree populates

### Backend Tools
- [ ] `list_dbus_services` API endpoint works
- [ ] `list_dbus_object_paths` API endpoint works
- [ ] `introspect_dbus_object` API endpoint works
- [ ] Returns valid JSON with D-Bus data

### Cache System
- [ ] Warmup script executes: `sudo warm-dbus-cache.sh`
- [ ] Cache file created: `/var/cache/dbus-introspection.db`
- [ ] Cache file is SQLite: `file /var/cache/dbus-introspection.db`

---

## Maintenance

### Starting MCP Server on Boot (Systemd Service)

See **Appendix A** below for systemd service file.

### Monitoring Logs

```bash
# MCP server log (if started with redirection)
tail -f /var/log/dbus-mcp.log

# Systemd timer log
sudo journalctl -u dbus-cache-warmup.timer -f

# Systemd service log
sudo journalctl -u dbus-mcp.service -f
```

### Updating After Git Changes

```bash
cd /home/user/operation-dbus

# Pull latest changes
git pull origin claude/ghostbfridge-non-nixos-improvements-018yb8CpSXQQuGGr8W1zz3TZ

# Rebuild
cargo build --release --bin dbus-mcp --features mcp

# If using Method A (staging):
# Rollback and redeploy affected component
sudo staging/op-dbus-stage-v2 rollback mcp-server
sudo staging/op-dbus-stage-v2 deploy mcp-server

# If using Method B (manual):
# Just copy new binary
sudo cp target/release/dbus-mcp /usr/local/bin/

# Restart server
sudo pkill dbus-mcp
sudo dbus-mcp --port 8080 --enable-web --web-root /var/www/op-dbus-mcp &
```

---

## Appendix A: Systemd Service for MCP Server

Create `/etc/systemd/system/dbus-mcp.service`:

```ini
[Unit]
Description=D-Bus MCP Server with Web UI
After=network.target dbus.service

[Service]
Type=simple
ExecStart=/usr/local/bin/dbus-mcp --port 8080 --enable-web --web-root /var/www/op-dbus-mcp
Restart=on-failure
RestartSec=5s
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable dbus-mcp.service
sudo systemctl start dbus-mcp.service
sudo systemctl status dbus-mcp.service
```

---

## Appendix B: Firewall Configuration

If accessing from remote machine:

```bash
# UFW
sudo ufw allow 8080/tcp
sudo ufw reload

# iptables
sudo iptables -A INPUT -p tcp --dport 8080 -j ACCEPT
sudo iptables-save > /etc/iptables/rules.v4
```

---

## Summary

This guide covers three deployment methods:

- **Method A (Recommended):** Staging tool with snapshots and organized structure
- **Method B (Manual):** Step-by-step installation with full control
- **Method C (Quick):** Run directly from repo for testing

All methods achieve the same end result:
- MCP server running on port 8080
- Discovery tools available via API
- Web UI accessible with tree view and workflow builder
- Cache warmup systemd timer running

Choose the method that best fits your needs and risk tolerance.
