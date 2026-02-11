# MCP Bridge: VS Code Extension Emulation

This setup replaces the old Antigravity path. The `op-mcp-proxy` direct mode calls
`cloudcode-pa.googleapis.com` and sends VS Code Cloud Code-style request headers.

## What It Uses

- Binary: `op-mcp-proxy`
- Mode: `DIRECT_MODE=1`
- Endpoint: `https://cloudcode-pa.googleapis.com/v1internal:generateContent`
- Auth source: `~/.gemini/oauth_creds.json` (Gemini CLI OAuth creds)

## Required Environment

```bash
export ENABLE_MCP_PROXY_PROVIDER=true
export LLM_PROVIDER=mcp-proxy
export OP_MCP_PROXY_BIN=op-mcp-proxy
export LLM_MODEL=gemini-2.5-flash

# Optional project override
export MCP_PROXY_GCLOUD_PROJECT=operation-dbus
```

## VS Code Emulation Headers

Defaults are now applied by `op-mcp-proxy` automatically. Override only if needed:

```bash
export MCP_PROXY_USER_AGENT="google-cloud-code-vscode/1.22.0 (GPN:Cloud Code for VS Code) vscode/1.85.0 (linux; x64)"
export MCP_PROXY_X_GOOG_API_CLIENT="gl-rust/1.76.0 gax/2.12.0 gapic/1.0.0"
export MCP_PROXY_ORIGIN="vscode://googlecloudtools.cloudcode"
export MCP_PROXY_REFERER="vscode://googlecloudtools.cloudcode"
export MCP_PROXY_X_CLIENT_DATA="eyJpc0lkZSI6dHJ1ZSwiaWRlVHlwZSI6InZzY29kZSIsImlkZVZlcnNpb24iOiIxLjg1LjAiLCJwbHVnaW5WZXJzaW9uIjoiMS4yMi4wIn0="
```

`x-goog-user-project` is sent by default from `MCP_PROXY_GCLOUD_PROJECT` (or discovered
project). To disable it:

```bash
export MCP_PROXY_SEND_X_GOOG_USER_PROJECT=false
```

## Quick Verify

1. Ensure Gemini CLI creds exist:
   `ls ~/.gemini/oauth_creds.json`
2. Run with proxy provider enabled.
3. Confirm logs show:
   `MCP bridge IDE emulation enabled`.

