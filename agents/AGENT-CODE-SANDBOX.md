# Code Sandbox Agent

Secure sandboxed code execution for Python and JavaScript.

## D-Bus Interface
`org.dbusmcp.Agent.CodeSandbox`

## Overview
Execute untrusted code safely in isolated environments with resource limits and timeout controls.

## Supported Languages
- **Python 3.x**: Full standard library
- **JavaScript (Node.js)**: V8 isolate with limited APIs
- **TypeScript**: Transpiled to JavaScript

## Tools

### execute_python
Run Python code in isolated environment.

**Input Schema:**
```json
{
  "code": "string (Python source code)",
  "timeout_ms": "integer (default: 5000, max: 30000)",
  "memory_limit_mb": "integer (default: 128, max: 512)",
  "allowed_modules": ["string (whitelist, default: stdlib only)"],
  "env_vars": {"key": "value (optional)"}
}
```

**Output Schema:**
```json
{
  "stdout": "string",
  "stderr": "string",
  "return_value": "any (JSON-serializable)",
  "execution_time_ms": "integer",
  "exit_code": "integer"
}
```

**Example:**
```json
{
  "code": "import math\nresult = math.sqrt(16)\nprint(f'Result: {result}')\nresult",
  "timeout_ms": 3000,
  "allowed_modules": ["math", "json"]
}
```

### execute_javascript
Run JavaScript/TypeScript code in V8 isolate.

**Input Schema:**
```json
{
  "code": "string (JS/TS source)",
  "language": "javascript|typescript",
  "timeout_ms": "integer (default: 5000)",
  "memory_limit_mb": "integer (default: 128)",
  "allowed_globals": ["string (e.g., 'console', 'JSON')"]
}
```

**Output Schema:**
```json
{
  "stdout": "string",
  "stderr": "string",
  "return_value": "any",
  "execution_time_ms": "integer",
  "error": "string (if execution failed)"
}
```

**Example:**
```json
{
  "code": "const data = [1, 2, 3, 4, 5];\nconst sum = data.reduce((a, b) => a + b, 0);\nconsole.log('Sum:', sum);\nsum;",
  "language": "javascript",
  "timeout_ms": 2000
}
```

### install_package
Install allowed packages into sandbox (Python only).

**Input Schema:**
```json
{
  "packages": ["string (package names)"],
  "pip_args": ["string (additional pip arguments, optional)"]
}
```

**Whitelist:** Only pre-approved packages allowed (numpy, pandas, requests, etc.)

## Security Features

### Sandboxing
- **No Network Access**: All network calls blocked
- **No Filesystem**: Virtual filesystem, no host access
- **No Process Spawning**: subprocess/exec disabled
- **Resource Limits**: CPU, memory, execution time

### Python Restrictions
- No `import os`, `import subprocess`
- No `eval()`, `exec()` on untrusted input
- No file operations (`open()` disabled)
- Restricted `__import__` mechanism

### JavaScript Restrictions
- No `require('fs')`, `require('child_process')`
- No `eval()`, `Function()` constructor
- No access to Node.js APIs
- V8 isolate with frozen globals

## Example Usage

### Via D-Bus
```bash
# Execute Python
busctl call org.dbusmcp.Agent.CodeSandbox \
  /org/dbusmcp/agent/sandbox_001 \
  org.dbusmcp.Agent.CodeSandbox \
  Execute s '{
    "task_type": "execute_python",
    "code": "def fibonacci(n):\n    if n <= 1:\n        return n\n    return fibonacci(n-1) + fibonacci(n-2)\n\nfibonacci(10)",
    "timeout_ms": 5000
  }'

# Execute JavaScript
busctl call org.dbusmcp.Agent.CodeSandbox \
  /org/dbusmcp/agent/sandbox_001 \
  org.dbusmcp.Agent.CodeSandbox \
  Execute s '{
    "task_type": "execute_javascript",
    "code": "JSON.stringify({result: Math.PI * 2})",
    "language": "javascript"
  }'
```

### Via MCP
```json
{
  "method": "tools/call",
  "params": {
    "name": "execute_python",
    "arguments": {
      "code": "import json\ndata = {'status': 'success', 'value': 42}\njson.dumps(data)",
      "timeout_ms": 3000,
      "allowed_modules": ["json"]
    }
  }
}
```

## Error Handling

### Timeout
```json
{
  "error": "ExecutionTimeout",
  "message": "Code execution exceeded 5000ms limit",
  "execution_time_ms": 5001
}
```

### Memory Limit
```json
{
  "error": "MemoryLimitExceeded",
  "message": "Process used more than 128MB RAM",
  "memory_used_mb": 145
}
```

### Syntax Error
```json
{
  "error": "SyntaxError",
  "stderr": "SyntaxError: invalid syntax",
  "line": 3,
  "column": 8
}
```

## Use Cases
- **Data Processing**: Run transformations on structured data
- **Algorithm Validation**: Test code snippets safely
- **Code Generation Testing**: Verify LLM-generated code
- **Mathematical Computation**: Complex calculations
- **JSON/Data Manipulation**: Parse, transform, validate

## Performance Notes
- First execution: ~100-500ms (environment initialization)
- Subsequent executions: ~10-100ms
- Memory overhead: ~50MB base + code requirements
- Parallel executions: Supported (isolated sandboxes)

## Configuration
- `MAX_CONCURRENT_EXECUTIONS`: Default 5
- `DEFAULT_TIMEOUT_MS`: Default 5000
- `ALLOWED_PACKAGES_FILE`: Path to package whitelist
- `SANDBOX_BACKEND`: docker|firecracker|gvisor (default: docker)
