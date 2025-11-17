# Vector Memory Agent

Semantic memory storage and retrieval using vector embeddings.

## D-Bus Interface
`org.dbusmcp.Agent.Memory.Vector`

## Overview
Stores and retrieves information using semantic similarity search via vector embeddings. Based on Qdrant MCP server specification.

## Tools

### store
Persist information with semantic indexing.

**Input Schema:**
```json
{
  "information": "string (content to store)",
  "metadata": {
    "source": "string (optional)",
    "timestamp": "string (optional)",
    "category": "string (optional)",
    "tags": ["string (optional)"]
  },
  "collection_name": "string (optional, default: memories)"
}
```

**Example:**
```json
{
  "information": "User prefers dark mode for all applications",
  "metadata": {
    "category": "preference",
    "timestamp": "2025-01-15T10:30:00Z"
  }
}
```

### find
Retrieve semantically relevant information.

**Input Schema:**
```json
{
  "query": "string (natural language search)",
  "collection_name": "string (optional)",
  "limit": "integer (optional, default: 5)"
}
```

**Output:**
```json
{
  "results": [
    {
      "information": "string",
      "score": "float (similarity)",
      "metadata": {}
    }
  ]
}
```

**Example:**
```json
{
  "query": "What are the user's UI preferences?",
  "limit": 3
}
```

## Configuration

### Environment Variables
- `QDRANT_URL`: Vector database endpoint (default: http://localhost:6333)
- `COLLECTION_NAME`: Default collection (default: memories)
- `EMBEDDING_MODEL`: FastEmbed model (default: BAAI/bge-small-en-v1.5)
- `TOOL_STORE_DESCRIPTION`: Custom description for store tool
- `TOOL_FIND_DESCRIPTION`: Custom description for find tool

## Features
- **Semantic Search**: Find information by meaning, not keywords
- **Auto-Collection**: Collections created automatically if absent
- **Metadata Filtering**: Optional structured data alongside content
- **Similarity Scoring**: Confidence metrics for each result

## Example Usage

### Via D-Bus
```bash
# Store information
busctl call org.dbusmcp.Agent.Memory.Vector \
  /org/dbusmcp/agent/vector_001 \
  org.dbusmcp.Agent.Memory.Vector \
  Execute s '{
    "task_type": "store",
    "information": "Client wants weekly progress reports every Monday",
    "metadata": {"category": "workflow"}
  }'

# Semantic search
busctl call org.dbusmcp.Agent.Memory.Vector \
  /org/dbusmcp/agent/vector_001 \
  org.dbusmcp.Agent.Memory.Vector \
  Execute s '{
    "task_type": "find",
    "query": "When should I send status updates?",
    "limit": 2
  }'
```

### Via MCP
```json
{
  "method": "tools/call",
  "params": {
    "name": "vector_find",
    "arguments": {
      "query": "code snippets for error handling",
      "collection_name": "snippets",
      "limit": 5
    }
  }
}
```

## Storage Backend
- **Qdrant Vector Database** (recommended)
- Auto-embedding with FastEmbed
- Persistent storage across sessions
- Scalable to millions of vectors

## Security Considerations
- Network access required for Qdrant connection
- API key authentication supported via QDRANT_API_KEY
- Embedding models run locally (no external API)
- Collection-level access control

## Use Cases
- **Code Snippet Library**: Find relevant examples semantically
- **Conversation History**: Retrieve past discussions by topic
- **Documentation Search**: Locate related information
- **Preference Recall**: Remember user choices contextually
- **Knowledge Base**: Build searchable information repository

## Performance Notes
- First query may be slower (model loading)
- Subsequent queries: ~50-200ms
- Embedding generation: ~10-50ms per document
- Collection size impacts query time minimally
