# op-dbus Plugin Ideas - Real World Examples

## Infrastructure & System Management

1. **sessions** ✅ (IMPLEMENTED)
   - Manage systemd-logind sessions
   - Ban/allow specific users on specific TTYs
   - Example: Prevent root login on tty1

2. **firewall**
   - UFW/iptables/nftables rules management
   - Declarative port allow/deny
   - IP whitelist/blacklist
   - Example: Allow only SSH from specific IPs

3. **docker-images**
   - Ensure specific Docker images are present/absent
   - Pull from registries
   - Prune unused images
   - Example: Always have postgres:15 available

4. **docker-containers**
   - Running container state management
   - Start/stop containers by name
   - Network attachments
   - Example: Keep nginx container running with specific ports

5. **btrfs-subvolumes**
   - Manage BTRFS subvolumes
   - Snapshots and retention
   - Quota management
   - Example: Daily snapshots with 7-day retention

6. **zfs-datasets**
   - ZFS dataset creation/destruction
   - Snapshot management
   - Compression and dedup settings
   - Example: Ensure pool/backup dataset exists

7. **lvm-volumes**
   - Logical volume management
   - Size enforcement
   - Thin provisioning
   - Example: Ensure /dev/vg0/data is 100GB

8. **mounts**
   - Filesystem mount state
   - NFS, CIFS, local mounts
   - Mount options enforcement
   - Example: Ensure NAS is mounted at /mnt/backup

9. **swap**
   - Swap file/partition management
   - Swappiness tuning
   - Zswap configuration
   - Example: Maintain 8GB swap with swappiness=10

10. **cron**
    - Crontab entry management
    - System and user crontabs
    - Job scheduling
    - Example: Daily backup at 2am

## Hardware & Devices

11. **pci-devices**
    - PCI device state monitoring
    - Driver binding/unbinding
    - IOMMU group management
    - Example: Ensure GPU is bound to vfio-pci

12. **usb-devices**
    - USB device authorization
    - Power management
    - Auto-suspend settings
    - Example: Block all USB storage devices

13. **block-devices**
    - Block device scheduler
    - Queue depth settings
    - Read-ahead configuration
    - Example: Set all SSDs to noop scheduler

14. **network-cards**
    - NIC offload settings
    - Ring buffer sizes
    - Interrupt coalescing
    - Example: Enable TSO on eth0

15. **gpu**
    - GPU mode (compute/graphics)
    - Power limits
    - Frequency capping
    - Example: Set GPU power limit to 250W

## Security & Access Control

16. **selinux**
    - SELinux policy management
    - Boolean toggles
    - Context enforcement
    - Example: Enable httpd_can_network_connect

17. **apparmor**
    - AppArmor profile management
    - Enforce/complain mode
    - Profile loading
    - Example: Enforce docker-default profile

18. **sudo-rules**
    - /etc/sudoers.d/ management
    - NOPASSWD rules
    - Command aliases
    - Example: Allow devs to restart nginx

19. **ssh-keys**
    - authorized_keys management
    - Per-user SSH key deployment
    - Key rotation
    - Example: Deploy team keys to all servers

20. **pam-limits**
    - /etc/security/limits.conf management
    - ulimit settings per user/group
    - nofile, nproc limits
    - Example: Set nofile=65536 for postgres user

## Network Configuration

21. **dns-resolver**
    - /etc/resolv.conf management
    - Nameserver configuration
    - Search domains
    - Example: Use 1.1.1.1 and 8.8.8.8

22. **hosts-file**
    - /etc/hosts entries
    - Static hostname mappings
    - Blocklist management
    - Example: Block ads via hosts file

23. **routing-tables**
    - Static route management
    - Policy routing rules
    - Default gateway
    - Example: Route 10.0.0.0/8 via VPN

24. **ipsec-tunnels**
    - IPsec VPN tunnel state
    - StrongSwan configuration
    - Tunnel keep-alive
    - Example: Maintain tunnel to remote office

25. **wireguard**
    - WireGuard interface management
    - Peer configuration
    - Allowed IPs
    - Example: VPN mesh between servers

## Services & Daemons

26. **systemd-timers**
    - Systemd timer units
    - OnCalendar scheduling
    - Timer activation
    - Example: Weekly log rotation timer

27. **docker-compose**
    - Docker Compose stack state
    - Multi-container applications
    - Stack updates
    - Example: Keep monitoring stack running

28. **kubernetes-pods**
    - Pod state on single-node k8s
    - Local kubectl operations
    - Namespace management
    - Example: Ensure metrics-server pod exists

29. **samba-shares**
    - Samba share configuration
    - smb.conf management
    - User access control
    - Example: Share /data as "backup" read-only

30. **nfs-exports**
    - NFS export management
    - /etc/exports configuration
    - Client access rules
    - Example: Export /backup to 192.168.1.0/24

## Package & Software Management

31. **apt-packages**
    - Debian/Ubuntu package state
    - Ensure installed/removed
    - Version pinning
    - Example: Always have latest nginx

32. **pip-packages**
    - Python package management
    - Virtualenv packages
    - Global vs user installs
    - Example: Ensure ansible==2.10

33. **npm-packages**
    - Node.js global packages
    - Version management
    - Registry configuration
    - Example: Keep pm2 installed globally

34. **snap-packages**
    - Snap package state
    - Channel management (stable/edge)
    - Auto-refresh settings
    - Example: Install microk8s from stable

35. **flatpak-apps**
    - Flatpak application state
    - Runtime management
    - Remote configuration
    - Example: Install Spotify flatpak

## System Tuning

36. **sysctl**
    - Kernel parameter management
    - /etc/sysctl.d/ configuration
    - Runtime tuning
    - Example: Set net.ipv4.ip_forward=1

37. **grub-config**
    - GRUB bootloader settings
    - Kernel parameters
    - Default boot entry
    - Example: Add nomodeset to kernel cmdline

38. **hugepages**
    - Huge page allocation
    - Size and quantity
    - NUMA node distribution
    - Example: Reserve 1GB huge pages

39. **cpu-governor**
    - CPU frequency scaling
    - Governor per-core
    - Turbo boost control
    - Example: Set all cores to performance

40. **io-scheduler**
    - I/O scheduler per device
    - Scheduler parameters
    - Device-specific tuning
    - Example: Set nvme* to none

## User & Authentication

41. **users**
    - Local user account management
    - UID/GID assignment
    - Home directory creation
    - Example: Ensure backup user exists

42. **groups**
    - Group membership management
    - GID assignment
    - Supplementary groups
    - Example: Add user to docker group

43. **ldap-config**
    - LDAP client configuration
    - NSS/PAM integration
    - Search base settings
    - Example: Connect to corporate LDAP

44. **kerberos**
    - Kerberos client setup
    - krb5.conf management
    - Keytab deployment
    - Example: Join AD domain

45. **certificates**
    - SSL/TLS certificate deployment
    - CA bundle updates
    - Certificate expiry tracking
    - Example: Deploy Let's Encrypt certs

## Monitoring & Logging

46. **rsyslog-rules**
    - Rsyslog configuration
    - Log forwarding rules
    - Facility/priority filters
    - Example: Forward auth.log to SIEM

47. **logrotate**
    - Log rotation configuration
    - Retention policies
    - Compression settings
    - Example: Rotate nginx logs daily

48. **audit-rules**
    - Linux audit framework rules
    - File watch rules
    - Syscall auditing
    - Example: Audit all /etc changes

49. **node-exporter**
    - Prometheus node exporter
    - Metrics collection
    - Textfile collector
    - Example: Expose custom metrics

50. **snmp-config**
    - SNMP agent configuration
    - Community strings
    - OID exposure
    - Example: Allow monitoring from NMS

## Bonus Ideas

51. **time-config**
    - Timezone setting
    - NTP server configuration
    - Chrony/systemd-timesyncd
    - Example: Use time.google.com

52. **locale**
    - System locale settings
    - Available locales
    - Default language
    - Example: Set en_US.UTF-8

53. **hostname**
    - System hostname
    - Static hostname
    - Pretty hostname
    - Example: Set hostname to web-01.prod

54. **kernel-modules**
    - Module loading/blacklisting
    - Module parameters
    - initramfs integration
    - Example: Blacklist nouveau, load nvidia

55. **fstab-entries**
    - /etc/fstab management
    - Persistent mount configuration
    - Mount options
    - Example: Add /dev/sdb1 to /data

56. **bridge-networks**
    - Linux bridge creation
    - Bridge port assignment
    - VLAN configuration
    - Example: Create br0 with eth0,eth1

57. **bond-interfaces**
    - Network bonding/teaming
    - Bond mode configuration
    - Slave interfaces
    - Example: Bond eth0+eth1 in 802.3ad

58. **vlan-interfaces**
    - VLAN tagging
    - Sub-interface creation
    - VLAN ID assignment
    - Example: Create eth0.100 for VLAN 100

59. **openvpn**
    - OpenVPN client/server state
    - Configuration deployment
    - Connection monitoring
    - Example: Maintain VPN to HQ

60. **fail2ban**
    - Fail2ban jail configuration
    - Ban/unban rules
    - Filter customization
    - Example: Protect SSH with 5-try limit

## Priority Recommendations for Hourly Generation

**High Value / Common Use Cases:**
1. firewall
2. docker-containers
3. cron
4. users
5. ssh-keys
6. apt-packages
7. sysctl
8. mounts
9. systemd-timers
10. hosts-file

**Infrastructure Automation:**
11. wireguard
12. btrfs-subvolumes
13. certificates
14. routing-tables
15. rsyslog-rules

**Security Focused:**
16. sudo-rules
17. selinux
18. audit-rules
19. pam-limits
20. fail2ban
