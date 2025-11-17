# Knowledge Graph Memory Agent

Persistent memory system using a local knowledge graph to remember information across conversations.

## D-Bus Interface
`org.dbusmcp.Agent.Memory.Graph`

## Overview
Manages entities, relations, and observations in a knowledge graph for long-term memory. Based on the official MCP memory server specification.

## Tools

### create_entities
Create new entities in the knowledge graph.

**Input Schema:**
```json
{
  "entities": [
    {
      "name": "string (unique identifier)",
      "entityType": "string (person|organization|place|concept)",
      "observations": ["string (discrete facts)"]
    }
  ]
}
```

### create_relations
Create directed relationships between entities.

**Input Schema:**
```json
{
  "relations": [
    {
      "from": "string (source entity name)",
      "to": "string (target entity name)",
      "relationType": "string (active voice descriptor)"
    }
  ]
}
```

### add_observations
Append new facts to existing entities.

**Input Schema:**
```json
{
  "observations": [
    {
      "entityName": "string",
      "contents": ["string (new facts to add)"]
    }
  ]
}
```

### delete_entities
Remove entities and cascade-delete their relations.

**Input Schema:**
```json
{
  "entityNames": ["string"]
}
```

### delete_observations
Remove specific observations from entities.

**Input Schema:**
```json
{
  "deletions": [
    {
      "entityName": "string",
      "observations": ["string (facts to remove)"]
    }
  ]
}
```

### delete_relations
Remove specific relationships.

**Input Schema:**
```json
{
  "relations": [
    {
      "from": "string",
      "to": "string",
      "relationType": "string"
    }
  ]
}
```

### read_graph
Retrieve the complete knowledge graph structure.

**Input:** None

**Output:**
```json
{
  "entities": [...],
  "relations": [...]
}
```

### search_nodes
Find entities by name, type, or observation content.

**Input Schema:**
```json
{
  "query": "string (search term)"
}
```

### open_nodes
Retrieve specific nodes and their relationships.

**Input Schema:**
```json
{
  "names": ["string (entity names)"]
}
```

## Storage
- Default: `memory.jsonl` in working directory
- Configurable via `MEMORY_FILE_PATH` environment variable

## Example Usage

### Via D-Bus (busctl)
```bash
# Create entity
busctl call org.dbusmcp.Agent.Memory.Graph \
  /org/dbusmcp/agent/memory_graph_001 \
  org.dbusmcp.Agent.Memory.Graph \
  Execute s '{
    "task_type": "create_entities",
    "entities": [{
      "name": "Alice",
      "entityType": "person",
      "observations": ["Lives in Seattle", "Software engineer"]
    }]
  }'

# Search
busctl call org.dbusmcp.Agent.Memory.Graph \
  /org/dbusmcp/agent/memory_graph_001 \
  org.dbusmcp.Agent.Memory.Graph \
  Execute s '{
    "task_type": "search_nodes",
    "query": "Seattle"
  }'
```

### Via MCP
```json
{
  "method": "tools/call",
  "params": {
    "name": "create_entities",
    "arguments": {
      "entities": [{
        "name": "ProjectX",
        "entityType": "project",
        "observations": ["Started in 2025", "Uses Rust"]
      }]
    }
  }
}
```

## Security Considerations
- File system access limited to configured memory file
- No network access
- Graph size unbounded - monitor disk usage
- Duplicate entity names rejected

## Use Cases
- User preference tracking
- Project context retention
- Relationship mapping
- Long-term conversation memory
- Cross-session information recall
