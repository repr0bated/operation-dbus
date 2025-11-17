# File Agent Specification

**D-Bus Interface**: `org.dbusmcp.Agent.File`
**Agent Type**: `file`
**Purpose**: Secure file operations with path validation

## Task Format

```json
{
  "type": "file",
  "operation": "read",
  "path": "/tmp/myfile.txt",
  "content": null,
  "recursive": false
}
```

## Supported Operations

- `read` - Read file contents
- `write` - Write content to file
- `delete` - Delete file or directory
- `exists` - Check if path exists
- `list` - List directory contents
- `mkdir` - Create directory

## Security Model

### Allowed Directories
- `/home`
- `/tmp`
- `/var/log`
- `/opt`

### Forbidden Directories
- `/etc`, `/root`, `/boot`, `/sys`, `/proc`, `/dev`
- `/usr/bin`, `/usr/sbin`, `/bin`, `/sbin`

### Forbidden Files
- SSH keys (`.ssh/id_rsa`, `.ssh/authorized_keys`)
- Shell history (`.bash_history`, `.zsh_history`)
- System files (`shadow`, `passwd`, `sudoers`)
- Secrets (`.env`, credentials)

### Limits
- **Max file size**: 10 MB
- **Max path length**: 4096 characters

## Usage Examples

```bash
# Read file
busctl call org.dbusmcp.Agent.File.{id} /org/dbusmcp/Agent/File/{id} \
  org.dbusmcp.Agent.File Execute s \
  '{"type":"file","operation":"read","path":"/tmp/test.txt"}'

# Write file
busctl call org.dbusmcp.Agent.File.{id} /org/dbusmcp/Agent/File/{id} \
  org.dbusmcp.Agent.File Execute s \
  '{"type":"file","operation":"write","path":"/tmp/test.txt","content":"Hello World"}'

# List directory
busctl call org.dbusmcp.Agent.File.{id} /org/dbusmcp/Agent/File/{id} \
  org.dbusmcp.Agent.File Execute s \
  '{"type":"file","operation":"list","path":"/tmp"}'
```

## Response Format

```json
{
  "success": true,
  "operation": "read",
  "path": "/tmp/test.txt",
  "data": "Hello World",
  "error": null
}
```

## Environment Variables

- `FILE_AGENT_BASE_DIR` - Override base directory (default: `/tmp/file-agent`)
