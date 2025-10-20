# op-dbus Deployment Checklist

## Pre-Deployment

- [ ] Review system current state
  ```bash
  ovs-vsctl show
  ip addr show
  ip route show
  systemctl status openvswitch-switch
  ```

- [ ] Backup current network config
  ```bash
  cp /etc/network/interfaces /etc/network/interfaces.backup
  ovs-vsctl show > /tmp/ovs-backup.txt
  ip addr > /tmp/ip-backup.txt
  ip route > /tmp/route-backup.txt
  ```

## Build & Install

- [ ] Build binary
  ```bash
  cd /git/op-dbus
  cargo build --release
  ```

- [ ] Verify binary exists
  ```bash
  ls -lh target/release/op-dbus
  ./target/release/op-dbus --version
  ```

- [ ] Install system-wide
  ```bash
  sudo ./install.sh
  ```

- [ ] Verify installation
  ```bash
  which op-dbus
  op-dbus --version
  ```

## Configuration

- [ ] Edit state file with YOUR network config
  ```bash
  sudo nano /etc/op-dbus/state.json
  ```

- [ ] Verify JSON syntax
  ```bash
  jq . /etc/op-dbus/state.json
  ```

## Safe Testing (READ-ONLY)

- [ ] Test query all
  ```bash
  sudo op-dbus query
  ```

- [ ] Test query network
  ```bash
  sudo op-dbus query --plugin net
  ```

- [ ] Test diff (what would change)
  ```bash
  sudo op-dbus diff /etc/op-dbus/state.json
  ```

- [ ] Review diff output carefully!
  - Does it match what you expect?
  - Are the IP addresses correct?
  - Is the gateway correct?
  - Are the interfaces correct?

## Apply State (MAKES CHANGES!)

⚠️ **WARNING: Network changes can cause 20-minute downtime if wrong!**

- [ ] Ensure you have console/IPMI access (in case network breaks)

- [ ] Apply state manually (DO NOT use systemd yet)
  ```bash
  sudo op-dbus apply /etc/op-dbus/state.json
  ```

- [ ] Verify network still works
  ```bash
  ping -c 3 8.8.8.8
  ping -c 3 google.com
  ```

- [ ] Verify OVS bridge
  ```bash
  ovs-vsctl show
  ip addr show vmbr0
  ip route show
  ```

- [ ] Test from another machine
  ```bash
  ssh user@your-server-ip
  ```

## Enable Service (After Manual Test Success)

- [ ] Enable service
  ```bash
  sudo systemctl enable op-dbus
  ```

- [ ] Start service
  ```bash
  sudo systemctl start op-dbus
  ```

- [ ] Check status
  ```bash
  sudo systemctl status op-dbus
  ```

- [ ] Watch logs
  ```bash
  sudo journalctl -u op-dbus -f
  ```

## Post-Deployment Verification

- [ ] Network connectivity
  ```bash
  ping -c 3 8.8.8.8
  curl -I https://google.com
  ```

- [ ] OVS state matches desired
  ```bash
  sudo op-dbus query --plugin net
  ```

- [ ] Service survives reboot
  ```bash
  # Optional: sudo reboot
  # After reboot:
  sudo systemctl status op-dbus
  ```

## Rollback (If Things Go Wrong)

- [ ] Stop service
  ```bash
  sudo systemctl stop op-dbus
  sudo systemctl disable op-dbus
  ```

- [ ] Restore backup
  ```bash
  sudo cp /etc/network/interfaces.backup /etc/network/interfaces
  sudo systemctl restart networking
  ```

- [ ] Manual OVS restore
  ```bash
  # Recreate bridge manually if needed
  sudo ovs-vsctl add-br vmbr0
  sudo ovs-vsctl add-port vmbr0 ens1
  sudo ip addr add 80.209.240.244/25 dev vmbr0
  sudo ip link set vmbr0 up
  sudo ip route add default via 80.209.240.129
  ```

## Success Criteria

✅ Service running: `systemctl status op-dbus`
✅ Network working: `ping 8.8.8.8`
✅ State matches: `op-dbus query` == desired
✅ Logs clean: `journalctl -u op-dbus`
✅ No errors in journal
