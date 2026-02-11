# op-dbus Dinit Service

This document tracks the standalone `op-dbus` + `op-mcp-proxy` runtime setup for
Chimera Linux using `dinit` instead of `systemd`.

## Files in Repo

- `deploy/dinit/op-dbus`
- `deploy/dinit/op-dbus-dinit.sh`
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
- `/etc/dinit.d/boot.d/op-dbus` symlink
- `/usr/local/bin/op-dbus-dinit.sh`
- `/usr/local/bin/op-mcp-proxy-select3`
- `/etc/op-dbus/environment` (only if missing)

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
