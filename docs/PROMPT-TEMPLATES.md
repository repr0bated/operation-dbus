# MCP Prompt Templates and Context Patterns

## System Context Templates

### D-Bus Service Analysis
```
Analyze the D-Bus service {service_name}:
1. What interfaces does it expose?
2. What methods are available?
3. What properties can be read/written?
4. What signals does it emit?
5. What are common use cases?

Context: {service_introspection_data}
```

### System State Understanding
```
Current system state:
- Services running: {service_count}
- D-Bus objects: {object_count}
- Active agents: {agent_list}
- Recent events: {event_log}

Task: {user_query}
Analyze the current state and determine the best approach.
```

## Memory Management Patterns

### Short-Term Context (Current Session)
Store in conversation:
- Current task state
- Recently executed commands
- Active D-Bus connections
- Temporary variables

### Long-Term Memory (Persistent)
Store in D-Bus index:
- Service configurations
- Common query patterns
- User preferences
- System topology

### Episodic Memory (Event-Based)
Store in BTRFS snapshots:
- State before/after changes
- System configuration history
- Audit trail of modifications

## RAG (Retrieval-Augmented Generation) Patterns

### Pattern 1: D-Bus Service Lookup
```rust
// When user asks about a service, retrieve from index first
let service_info = dbus_index.get_service("org.freedesktop.NetworkManager");
// Then augment response with current live state
let live_state = introspect_live(service_info.name);
```

### Pattern 2: Historical Context
```rust
// Compare current state with previous snapshots
let current = build_index();
let snapshot = load_snapshot("@yesterday");
let diff = compute_diff(current, snapshot);
```

### Pattern 3: Semantic Search
```rust
// Find relevant documentation by semantic similarity
let query_embedding = embed_query(user_question);
let relevant_docs = resources.search_semantic(query_embedding);
```

## Agent Coordination Templates

### Multi-Agent Workflow
```
Coordinator Agent:
1. Analyze task: {task_description}
2. Break into subtasks: [{subtask_1}, {subtask_2}, ...]
3. Assign to specialized agents:
   - systemd_agent: {subtask_1}
   - network_agent: {subtask_2}
4. Aggregate results
5. Return unified response
```

### Agent Handoff Pattern
```
Current Agent: executor
Task State: {serialized_state}
Handoff Reason: {reason}
Next Agent: systemd
Context: {relevant_context}
```

## Context Window Management

### Summarization Strategy
```
When context exceeds threshold:
1. Summarize completed tasks
2. Keep recent 5 interactions verbatim
3. Extract key facts to context summary
4. Archive full history to D-Bus index
```

### Hierarchical Context
```
Level 1 (Always included): System state, available tools
Level 2 (Recent): Last 10 interactions
Level 3 (Relevant): Search results from index
Level 4 (Archived): BTRFS snapshot references
```

## Tool Call Templates

### D-Bus Method Invocation
```json
{
  "tool": "dbus_call",
  "service": "org.freedesktop.systemd1",
  "object": "/org/freedesktop/systemd1",
  "interface": "org.freedesktop.systemd1.Manager",
  "method": "ListUnits",
  "args": []
}
```

### State Transition
```json
{
  "tool": "apply_state",
  "diff": {
    "add": [{"type": "service", "name": "nginx", "state": "enabled"}],
    "remove": [],
    "modify": []
  },
  "dry_run": true
}
```

## Error Recovery Patterns

### Graceful Degradation
```
If primary method fails:
1. Try ObjectManager.GetManagedObjects (fast)
   ↓ fails
2. Try recursive introspection (slower)
   ↓ fails
3. Use cached index data (stale but available)
   ↓ fails
4. Ask user for manual intervention
```

### Retry with Exponential Backoff
```
Attempt 1: Immediate
Attempt 2: Wait 2s
Attempt 3: Wait 4s
Attempt 4: Wait 8s
Max attempts: 4
```

## Context Injection Strategies

### System Awareness Prompt
```
You are an AI assistant with direct access to:
- Complete D-Bus introspection index ({total_services} services)
- BTRFS snapshot history (last {snapshot_count} snapshots)
- System state manager with {plugin_count} plugins
- MCP tools for system automation

Current capabilities:
{tool_list}

Use these tools to provide accurate, actionable responses.
```

### Domain Expert Prompt
```
You are a {domain} expert with access to:
- Relevant documentation: {doc_list}
- System state for {domain}: {domain_state}
- Historical data: {relevant_snapshots}

Provide expert-level guidance while considering current system state.
```

## Continuous Learning Patterns

### Pattern Recognition
```
Track common queries:
- "How do I restart NetworkManager?" → systemctl restart NetworkManager
- "List all services" → dbus index query --category systemd
- "Show network config" → nmcli or ip addr

Build shortcut index for frequent operations.
```

### User Preference Learning
```
Observed patterns:
- User prefers systemctl over D-Bus API
- User always wants dry-run first
- User focuses on network services

Adapt responses accordingly.
```

## Multi-Modal Context

### Code + Explanation
```
Task: Enable service
Code: `systemctl enable nginx`
Explanation: This creates a symlink in systemd's wants directory
D-Bus equivalent: Call Enable method on systemd1.Manager
```

### Visual + Data
```
System Topology:
[ASCII diagram of D-Bus service relationships]

Data Structure:
{json_representation}
```

## Context Compression Techniques

### Key-Value Extraction
```
Full conversation: 500 tokens
Compressed facts:
- service_name: nginx
- desired_state: enabled
- current_state: disabled
- method: systemctl
Result: 50 tokens (90% reduction)
```

### Reference-Based Context
```
Instead of: [Full 1000-line introspection XML]
Use: Reference: dbus://service/org.freedesktop.NetworkManager
Retrieve on-demand from embedded resources
```
