# op-dbus Dinit Service

This document tracks the standalone `op-dbus` + `op-mcp-proxy` runtime setup for
Chimera Linux using `dinit` instead of `systemd`.

## Files in Repo

- `deploy/dinit/op-dbus`
- `deploy/dinit/op-session-bus`
- `deploy/dinit/op-ovsdb-bridge`
- `deploy/dinit/op-dbus-dinit.sh`
- `deploy/dinit/op-web-dinit.sh`
- `deploy/dinit/op-session-bus.sh`
- `deploy/dinit/op-ovsdb-bridge-start.sh`
- `deploy/dinit/op-mcp-proxy-select3`
- `deploy/dinit/environment.op-dbus.template`
- `deploy/dinit/install-op-dbus-dinit.sh`

## Install

```bash
cd /path/to/operation-dbus
doas ./deploy/dinit/install-op-dbus-dinit.sh
```

The installer writes:

- `/etc/dinit.d/op-dbus`
- `/etc/dinit.d/op-session-bus`
- `/etc/dinit.d/op-ovsdb-bridge`
- `/etc/dinit.d/boot.d/op-dbus` symlink
- `/etc/dinit.d/boot.d/op-session-bus` symlink
- `/etc/dinit.d/boot.d/op-ovsdb-bridge` symlink
- `/usr/local/bin/op-dbus-dinit.sh`
- `/usr/local/sbin/op-dbus-dinit.sh`
- `/usr/local/sbin/op-web-dinit.sh`
- `/usr/local/sbin/op-session-bus`
- `/etc/dinit.d/scripts/op-ovsdb-bridge-start.sh`
- `/usr/local/bin/op-mcp-proxy-select3`
- `/etc/op-dbus/environment` (only if missing)

## OVS Boot Protocol

`op-ovsdb-bridge` is idempotent at boot and uses `busctl` -> `org.opdbus` only:

- Creates `PRIVACY_BRIDGE_NAME` (default `ovsbr0`) if missing via `org.opdbus.OvsdbV1.CreateBridge`.
- Ensures `PRIVACY_UPLINK_PORT` is attached via `org.opdbus.OvsdbV1.AddPort`.
- Calls `org.opdbus.RtnetlinkV1.LinkUp` for bridge/uplink and managed interfaces.
- Optionally configures IPv4 address(es) with `RtnetlinkV1.AddIpv4Address`.
- Optionally configures default route with `RtnetlinkV1.AddDefaultRoute`.
- Runs mirror reconcile via `org.opdbus.v1` at `/org/opdbus/v1` (with legacy fallback).

Optional boot-time address/route environment keys:

- `PRIMARY_PUBLIC_IPV4_CIDR`
- `PRIMARY_PUBLIC_IPV4_IFACE` (default: `PRIVACY_UPLINK_PORT`)
- `SECONDARY_PUBLIC_IPV4_CIDR`
- `SECONDARY_PUBLIC_IPV4_IFACE` (default: `PRIVACY_BRIDGE_NAME`)
- `DEFAULT_IPV4_GATEWAY`
- `DEFAULT_IPV4_IFACE` (default: `PRIVACY_UPLINK_PORT`)

## Binary Paths

Install or update runtime binaries:

```bash
doas install -m 0755 target/release/op-dbus /usr/local/bin/op-dbus
doas install -m 0755 target/release/op-mcp-proxy /usr/local/bin/op-mcp-proxy
```

## Model Selection

`LLM_MODEL=auto` is constrained to Gemini 3 family:

- `gemini-3-flash`
- `gemini-3-pro`
- With preview mode enabled:
  - `gemini-3-flash-preview`
  - `gemini-3-pro-preview`

Selector thresholds are configured in `/etc/op-dbus/environment` with:

- `MCP_PROXY_AUTO_FLASH_MODEL`
- `MCP_PROXY_AUTO_PRO_MODEL`
- `MCP_PROXY_AUTO_PRO_THRESHOLD_CHARS`
- `MCP_PROXY_EXPERIMENTAL`

If `MCP_PROXY_EXPERIMENTAL` is not set, selector follows
`~/.gemini/settings.json` -> `general.previewFeatures`.

## Health Check

```bash
dinitctl status op-dbus
curl -fsS http://127.0.0.1:7010/api/health
```

## Reverse Proxy and TLS

Enable nginx at boot (dinit system instance):

```bash
doas ln -sfn ../nginx /etc/dinit.d/boot.d/nginx
doas dinitctl restart nginx || doas dinitctl start nginx
```

Install nginx config from repo:

```bash
doas install -m 0644 deploy/nginx/op-web-3etched.com.conf /etc/nginx/http.d/op-web-3etched.conf
doas nginx -t && doas nginx -s reload
```

Issue/expand cert to include dashboard:

```bash
doas certbot certonly --webroot -w /var/www/certbot \
  --cert-name 3tched.com \
  -d 3tched.com -d www.3tched.com -d dashboard.3tched.com --expand
doas dinitctl restart nginx
```
