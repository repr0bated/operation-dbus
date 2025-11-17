# Conversation Buffer Memory Agent

Manages conversation history and context across multiple memory strategies.

## D-Bus Interface
`org.dbusmcp.Agent.Memory.Buffer`

## Overview
Provides multiple memory management strategies for maintaining conversation context with token-efficient approaches.

## Memory Modes

### 1. Buffer (Full History)
Stores complete conversation history.

**Input Schema:**
```json
{
  "mode": "buffer",
  "action": "add|get|clear",
  "message": {
    "role": "user|assistant|system",
    "content": "string"
  }
}
```

**Characteristics:**
- Retains all messages sequentially
- Best for: Short conversations, full context needed
- Cost: Grows linearly with conversation length

### 2. Window (Recent N Messages)
Maintains only the last N messages.

**Input Schema:**
```json
{
  "mode": "window",
  "window_size": "integer (e.g., 10)",
  "action": "add|get|clear",
  "message": {
    "role": "string",
    "content": "string"
  }
}
```

**Characteristics:**
- Fixed memory size
- Best for: Long conversations, recent context sufficient
- Cost: Constant token usage

### 3. Summary (Compressed History)
Periodically summarizes conversation to maintain context efficiently.

**Input Schema:**
```json
{
  "mode": "summary",
  "max_token_limit": "integer (trigger threshold)",
  "action": "add|get|summarize|clear",
  "message": {
    "role": "string",
    "content": "string"
  }
}
```

**Characteristics:**
- Compresses old messages into summaries
- Best for: Very long conversations, context retention
- Cost: Periodic summarization overhead, then constant

### 4. Summary Buffer (Hybrid)
Combines summary and buffer approaches.

**Input Schema:**
```json
{
  "mode": "summary_buffer",
  "max_token_limit": "integer",
  "buffer_size": "integer (recent messages kept verbatim)",
  "action": "add|get|summarize|clear",
  "message": {
    "role": "string",
    "content": "string"
  }
}
```

**Characteristics:**
- Recent messages: Full detail
- Older messages: Summarized
- Best for: Balance between detail and efficiency
- Cost: Hybrid (summary overhead + recent buffer)

## Tools

### add_message
Add a new message to conversation history.

**Input Schema:**
```json
{
  "conversation_id": "string (session identifier)",
  "memory_config": {
    "mode": "buffer|window|summary|summary_buffer",
    "window_size": "integer (optional)",
    "max_token_limit": "integer (optional)"
  },
  "message": {
    "role": "user|assistant|system",
    "content": "string",
    "timestamp": "string (ISO 8601, optional)"
  }
}
```

### get_history
Retrieve conversation history.

**Input Schema:**
```json
{
  "conversation_id": "string",
  "format": "messages|text|tokens"
}
```

**Output:**
```json
{
  "conversation_id": "string",
  "messages": [...],
  "token_count": "integer",
  "summary": "string (if summary mode)"
}
```

### clear_history
Clear conversation history for a session.

**Input Schema:**
```json
{
  "conversation_id": "string"
}
```

### get_statistics
Get memory usage statistics.

**Output:**
```json
{
  "conversations": {
    "conv_id": {
      "message_count": "integer",
      "token_count": "integer",
      "mode": "string",
      "created_at": "string",
      "last_updated": "string"
    }
  }
}
```

## Example Usage

### Via D-Bus
```bash
# Add message with window mode
busctl call org.dbusmcp.Agent.Memory.Buffer \
  /org/dbusmcp/agent/buffer_001 \
  org.dbusmcp.Agent.Memory.Buffer \
  Execute s '{
    "task_type": "add_message",
    "conversation_id": "session_123",
    "memory_config": {
      "mode": "window",
      "window_size": 10
    },
    "message": {
      "role": "user",
      "content": "What were we discussing about the database?"
    }
  }'

# Get history
busctl call org.dbusmcp.Agent.Memory.Buffer \
  /org/dbusmcp/agent/buffer_001 \
  org.dbusmcp.Agent.Memory.Buffer \
  Execute s '{
    "task_type": "get_history",
    "conversation_id": "session_123",
    "format": "messages"
  }'
```

### Via MCP
```json
{
  "method": "tools/call",
  "params": {
    "name": "add_message",
    "arguments": {
      "conversation_id": "user_alice_session",
      "memory_config": {
        "mode": "summary_buffer",
        "max_token_limit": 2000,
        "buffer_size": 5
      },
      "message": {
        "role": "assistant",
        "content": "I've stored that preference for future reference."
      }
    }
  }
}
```

## Mode Selection Guide

| Use Case | Recommended Mode | Rationale |
|----------|-----------------|-----------|
| Short chat (< 10 messages) | `buffer` | Full context, low cost |
| Customer support | `window` (20-30) | Recent context sufficient |
| Long-running assistant | `summary_buffer` | Balance detail & efficiency |
| Document Q&A | `summary` | Compress context, retain key facts |
| Multi-session continuity | `summary` | Persistent compressed history |

## Storage
- In-memory by default
- Optional persistence via SQLite/JSON
- Configurable via `BUFFER_STORAGE_PATH`

## Security Considerations
- Conversation data stored unencrypted
- conversation_id acts as access key
- No automatic expiration (manual clear required)
- Memory usage grows with active conversations

## Performance
- **Buffer**: O(1) add, O(n) retrieval
- **Window**: O(1) add/retrieval
- **Summary**: O(n) summarization, O(1) retrieval
- **Token Counting**: ~1-5ms per message

## Integration Example
```python
# LangChain-style usage
memory = ConversationBufferMemory(
    mode="summary_buffer",
    max_token_limit=2000,
    buffer_size=5,
    conversation_id="session_abc"
)

memory.add_message("user", "Tell me about X")
memory.add_message("assistant", "X is...")

context = memory.get_history(format="text")
```
