# op-dbus Installation Testing Plan

## Testing Philosophy

**Objective**: Systematically validate the installation scripts and ensure op-dbus can be installed reliably across different scenarios.

**Approach**: Progressive testing - start simple, add complexity
- Unit tests (individual script functions)
- Integration tests (full installation flow)
- Edge case tests (failures, missing deps, etc.)
- Regression tests (ensure fixes don't break existing functionality)

---

## Test Suite Structure

### Phase 1: Build & Binary Testing
**Goal**: Verify the binary builds and runs correctly

1. **Build Test**
   - Clean build: `cargo clean && ./build.sh`
   - Verify binary exists: `test -f target/release/op-dbus`
   - Verify binary is executable: `test -x target/release/op-dbus`
   - Verify version command: `./target/release/op-dbus --version`
   - Verify help command: `./target/release/op-dbus --help`

2. **Feature Build Tests**
   - Default build (web feature)
   - MCP build: `cargo build --release --features mcp`
   - ML build: `cargo build --release --features ml`
   - All features: `cargo build --release --all-features`
   - Minimal build: `cargo build --release --no-default-features`

3. **Binary Size & Dependencies**
   - Check binary size (should be reasonable)
   - Check dynamic dependencies: `ldd target/release/op-dbus`
   - Verify no unexpected dependencies

### Phase 2: Dependency Installation Testing
**Goal**: Verify install-dependencies.sh works correctly

1. **Prerequisite Checks**
   - Test on system without openvswitch (should install it)
   - Test on system with openvswitch (should detect it)
   - Test Rust detection (with/without Rust installed)

2. **Platform Detection**
   - Test on Debian/Ubuntu
   - Test on different versions
   - Verify correct package manager used

3. **Service Status**
   - Verify OVS services start after installation
   - Verify OVSDB socket is created
   - Test OVS connectivity: `ovs-vsctl show`

4. **Optional Component Handling**
   - Test with user choosing to install netclient
   - Test with user declining netclient
   - Verify Proxmox detection works

### Phase 3: Installation Script Testing (install.sh)
**Goal**: Test the main installation process in all modes

#### Test 3.1: Agent-Only Mode
```bash
sudo ./install.sh --agent-only
```
**Expected Results:**
- Binary installed to /usr/local/bin/op-dbus
- Config directory created: /etc/op-dbus/
- Minimal state.json (systemd plugin only)
- Blockchain directories created (but not used)
- Service file created with minimal dependencies
- No OVS bridges created
- Service can start and stop

**Verification:**
```bash
sudo ./verify-installation.sh
test ! -f /etc/systemd/system/op-dbus.service || grep -q "After=network-online.target" /etc/systemd/system/op-dbus.service
op-dbus query --plugin systemd
```

#### Test 3.2: Standalone Mode
```bash
sudo ./install.sh --standalone
```
**Expected Results:**
- All agent-only components +
- State.json includes net plugin with ovsbr0 bridge
- OVS bridge "ovsbr0" created
- Service depends on openvswitch-switch.service
- Bridge visible in kernel: `ip link show ovsbr0`
- Can query network state: `op-dbus query --plugin net`

**Verification:**
```bash
sudo ./verify-installation.sh
ovs-vsctl list-br | grep -q "ovsbr0"
ip link show ovsbr0
op-dbus query --plugin net
```

#### Test 3.3: Full (Proxmox) Mode
```bash
sudo ./install.sh --full
```
**Expected Results:**
- All standalone components +
- State.json includes lxc plugin
- Two bridges created: ovsbr0, mesh
- LXC plugin registered
- Container management commands work

**Verification:**
```bash
sudo ./verify-installation.sh
ovs-vsctl list-br | grep -q "mesh"
op-dbus query --plugin lxc
op-dbus container list
```

#### Test 3.4: Interactive Mode Selection
```bash
sudo ./install.sh
# Interactively select mode 1, 2, or 3
```
**Expected Results:**
- Prompts user for mode selection
- Behaves identically to flag-based mode selection

#### Test 3.5: Introspection vs Template
**Test introspection path:**
```bash
sudo ./install.sh --standalone
# Choose "Y" for introspection
```
**Expected Results:**
- Uses `op-dbus init --introspect` to generate state
- Detects existing system configuration
- Creates realistic state.json

**Test template path:**
```bash
sudo ./install.sh --standalone
# Choose "n" for introspection
```
**Expected Results:**
- Generates template state.json
- Template matches selected mode
- Valid JSON syntax

#### Test 3.6: State Application
**Test successful apply:**
- State.json is valid
- `op-dbus apply` succeeds
- Changes are visible in system (bridges created, services started)

**Test dry-run:**
```bash
op-dbus apply /etc/op-dbus/state.json --dry-run
```
- Shows what would be applied
- Makes no actual changes

**Test skip apply:**
- Choose "n" when asked to apply
- State file exists but not applied
- Can manually apply later

### Phase 4: Verification Script Testing
**Goal**: Test verify-installation.sh catches issues

#### Test 4.1: Successful Installation Verification
```bash
# After successful install
sudo ./verify-installation.sh
```
**Expected**: All checks pass, exit code 0

#### Test 4.2: Missing Components Detection
**Test missing binary:**
```bash
sudo mv /usr/local/bin/op-dbus /tmp/
sudo ./verify-installation.sh
```
**Expected**: Fails binary check, reports error

**Test missing state file:**
```bash
sudo mv /etc/op-dbus/state.json /tmp/
sudo ./verify-installation.sh
```
**Expected**: Fails state file check

**Test invalid JSON:**
```bash
echo "invalid json" | sudo tee /etc/op-dbus/state.json
sudo ./verify-installation.sh
```
**Expected**: Fails JSON validation

#### Test 4.3: Service Status Detection
**Test stopped service:**
```bash
sudo systemctl stop op-dbus.service
sudo ./verify-installation.sh
```
**Expected**: Reports service not running (info, not error)

**Test failed service:**
```bash
# Break the service somehow
sudo systemctl start op-dbus.service
sudo ./verify-installation.sh
```
**Expected**: Reports service status

### Phase 5: Functional Testing
**Goal**: Verify op-dbus commands work post-installation

#### Test 5.1: Core Commands
```bash
op-dbus --version
op-dbus --help
op-dbus doctor
op-dbus query
op-dbus query --plugin net
op-dbus query --plugin systemd
op-dbus introspect --pretty
```

#### Test 5.2: State Management
```bash
# Show diff (should show no changes after fresh apply)
sudo op-dbus diff /etc/op-dbus/state.json

# Modify state, show diff
# Edit state.json to add a change
sudo op-dbus diff /etc/op-dbus/state.json

# Apply changes
sudo op-dbus apply /etc/op-dbus/state.json

# Verify changes applied
sudo op-dbus query
```

#### Test 5.3: Blockchain/Footprints
```bash
# Check blockchain directory
ls -la /var/lib/op-dbus/blockchain/timing/
ls -la /var/lib/op-dbus/blockchain/vectors/

# Blockchain commands
op-dbus blockchain list
op-dbus verify
```

#### Test 5.4: Container Commands (Full mode only)
```bash
op-dbus container list
# Should work even with no containers
```

### Phase 6: Edge Case & Error Handling
**Goal**: Test failure scenarios and recovery

#### Test 6.1: Re-installation
```bash
# Install once
sudo ./install.sh --standalone

# Install again (should handle existing installation)
sudo ./install.sh --standalone
```
**Expected**: Prompts for overwrite, handles gracefully

#### Test 6.2: Missing Prerequisites
```bash
# Uninstall OVS
sudo apt remove openvswitch-switch

# Try to install
sudo ./install.sh
```
**Expected**: Clear error message, suggests running install-dependencies.sh

#### Test 6.3: Permissions Issues
```bash
# Run without sudo
./install.sh
```
**Expected**: Clear error about needing root

#### Test 6.4: Disk Space
- Test with low disk space
- Expected: Graceful failure or warning

#### Test 6.5: Service Conflicts
- Test with port conflicts
- Test with socket conflicts
- Expected: Clear error messages

### Phase 7: Upgrade/Migration Testing
**Goal**: Test upgrading from old install to new

#### Test 7.1: Upgrade from Old Install Script
```bash
# Install with old script
sudo ./install.sh.original

# Upgrade with new script
sudo ./install.sh
```
**Expected**: Preserves data, updates components

### Phase 8: Uninstallation Testing
**Goal**: Verify uninstall.sh works correctly

#### Test 8.1: Complete Uninstall
```bash
sudo ./install.sh --standalone
sudo ./uninstall.sh
```
**Expected**:
- Prompts for confirmation
- Removes binary
- Removes service
- Optional: removes data (prompts)
- Optional: removes bridges (prompts)

#### Test 8.2: Reinstall After Uninstall
```bash
sudo ./uninstall.sh
sudo ./install.sh --standalone
```
**Expected**: Clean installation works

---

## Test Execution Strategy

### Manual Testing (Current Phase)
1. Execute each test manually
2. Document results
3. Fix issues found
4. Re-test after fixes
5. Iterate until all tests pass

### Automated Testing (Future)
Create test automation scripts:
- `test-install.sh` - Runs all installation tests
- `test-verify.sh` - Runs verification tests
- `test-functional.sh` - Runs functional tests
- `test-all.sh` - Runs complete test suite

### Continuous Testing
- Test on every script change
- Test before committing changes
- Regression test suite for known issues

---

## Test Documentation

### Test Results Log Format
```
Test: [Test Name]
Date: [YYYY-MM-DD HH:MM]
Status: [PASS/FAIL/SKIP]
Duration: [seconds]
Notes: [observations]
Issues Found: [list of issues]
```

### Issue Tracking
When issues are found:
1. Document the issue
2. Create a fix
3. Update the script
4. Re-run the test
5. Verify fix works
6. Run regression tests

---

## Success Criteria

Installation is considered successful when:
- ✅ All three modes install without errors
- ✅ verify-installation.sh passes all checks
- ✅ op-dbus commands work correctly
- ✅ Services start and stop properly
- ✅ State can be queried and applied
- ✅ Blockchain footprints are created
- ✅ Uninstall works cleanly
- ✅ Reinstall works after uninstall

---

## Current Testing Status

Last Updated: 2025-11-08

- [ ] Phase 1: Build & Binary Testing
- [ ] Phase 2: Dependency Installation Testing
- [ ] Phase 3: Installation Script Testing
- [ ] Phase 4: Verification Script Testing
- [ ] Phase 5: Functional Testing
- [ ] Phase 6: Edge Case Testing
- [ ] Phase 7: Upgrade Testing
- [ ] Phase 8: Uninstallation Testing

**Next Steps:**
1. Complete binary build
2. Begin Phase 1 testing
3. Document results
4. Fix issues as they arise
5. Progress through phases systematically
