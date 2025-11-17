# Executor Agent Specification

**D-Bus Interface**: `org.dbusmcp.Agent.Executor`
**Agent Type**: `executor`
**Purpose**: Secure command execution with whitelist-based security

## Overview

The Executor Agent provides controlled execution of shell commands through D-Bus. It implements strict security controls including command whitelisting, input validation, and timeout enforcement.

## Security Model

### Allowed Commands (Whitelist)
```
ls, cat, grep, ps, top, df, du, free, uptime, whoami, date, hostname,
pwd, echo, wc, sort, head, tail
```

### Forbidden Characters
```
$ ` ; & | > < ( ) { } \n \r
```

### Limits
- **Max command length**: 1024 characters
- **Default timeout**: 30 seconds
- **Max timeout**: 300 seconds (5 minutes)

## D-Bus Methods

### execute(task_json: String) → String

Execute a command with security validation.

**Task JSON Format**:
```json
{
  "type": "execute",
  "command": "ls",
  "args": ["-la", "/tmp"],
  "timeout": 30,
  "working_dir": "/home/user"
}
```

**Response Format**:
```json
{
  "success": true,
  "exit_code": 0,
  "stdout": "total 24\ndrwxrwxrwt 10 root root 4096 ...\n",
  "stderr": "",
  "error": null
}
```

### get_status() → String

Returns agent status and configuration.

**Response**:
```
"Executor agent {agent_id} is running"
```

## D-Bus Signals

### task_completed(result: String)

Emitted when task execution completes (foreground or background).

## Usage Examples

### Via busctl
```bash
# List files
busctl call org.dbusmcp.Agent.Executor.{id} \
  /org/dbusmcp/Agent/Executor/{id} \
  org.dbusmcp.Agent.Executor \
  Execute s '{"type":"execute","command":"ls","args":["-la","/tmp"]}'

# Check uptime
busctl call org.dbusmcp.Agent.Executor.{id} \
  /org/dbusmcp/Agent/Executor/{id} \
  org.dbusmcp.Agent.Executor \
  Execute s '{"type":"execute","command":"uptime"}'
```

### Via MCP Tool
```json
{
  "tool": "executor_execute",
  "arguments": {
    "command": "ps",
    "args": ["aux"],
    "timeout": 10
  }
}
```

## Error Handling

### Validation Errors
- **Command too long**: `"Command exceeds maximum length of 1024 characters"`
- **Empty command**: `"Command cannot be empty"`
- **Forbidden character**: `"Command contains forbidden character: '$'"`
- **Not whitelisted**: `"Command 'rm' is not in the whitelist"`

### Execution Errors
- **Timeout**: `"Command timed out after {n} seconds"`
- **Execution failure**: Returns non-zero exit code + stderr
- **Working directory invalid**: `"Invalid working directory: {path}"`

## Use Cases

1. **System Diagnostics**
   - Check disk usage: `df -h`
   - View running processes: `ps aux`
   - Monitor system load: `uptime`, `top`

2. **File Operations**
   - List directory contents: `ls -la`
   - Read file contents: `cat /var/log/syslog`
   - Search files: `grep -r "error" /var/log`

3. **Quick Checks**
   - Current date/time: `date`
   - Current user: `whoami`
   - Hostname: `hostname`

## Limitations

1. **No privilege escalation**: Cannot run sudo/su commands
2. **No shell features**: No pipes, redirects, or command substitution
3. **Whitelist only**: Limited to predefined safe commands
4. **No background jobs**: All commands run in foreground with timeout

## Best Practices

1. **Always specify timeout** for potentially long-running commands
2. **Validate output size** before requesting large command outputs
3. **Use specific arguments** rather than wildcards when possible
4. **Check exit codes** to detect command failures
5. **Monitor stdout/stderr** for error messages

## Integration with Other Agents

- **File Agent**: Use executor to read files, then file agent to write
- **Network Agent**: Get network info via executor (`ip addr`), analyze with network agent
- **Systemd Agent**: Check service status via executor, manage via systemd agent

## Configuration

### Environment Variables
- None currently supported

### Future Enhancements
- **Configurable whitelist**: Allow custom command lists per deployment
- **Resource limits**: CPU/memory constraints for commands
- **Audit logging**: Detailed command execution logs
- **Background execution**: Support for long-running async commands
