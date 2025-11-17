# AI Memory and Context Management Patterns

## Overview

This document describes memory patterns for AI agents working with the op-dbus system, focusing on efficient context management, state persistence, and knowledge retrieval.

## Memory Hierarchy

### L1: Working Memory (Immediate Context)
- **Scope**: Current conversation/task
- **Size**: ~4K tokens
- **Lifetime**: Single session
- **Storage**: In-memory conversation buffer

**Contents**:
- Current user query
- Last 3-5 interactions
- Active tool call state
- Temporary variables

**Example**:
```json
{
  "current_task": "Restart NetworkManager",
  "context": {
    "service": "NetworkManager",
    "method": "systemctl",
    "state": "checking_status"
  },
  "recent_outputs": ["NetworkManager is running", "No errors in journal"]
}
```

### L2: Session Memory (Task Context)
- **Scope**: Current session
- **Size**: ~16K tokens
- **Lifetime**: Until session end
- **Storage**: Session state in D-Bus index

**Contents**:
- Complete conversation history
- All tool call results
- Discovered system state
- User preferences for session

**Persistence**:
```rust
// Save session state to D-Bus index
session_manager.save_state(&SessionState {
    id: session_id,
    started: timestamp,
    conversations: conversation_log,
    system_state: discovered_services,
    user_context: preferences,
});
```

### L3: Episodic Memory (Historical Events)
- **Scope**: Cross-session events
- **Size**: Unlimited (BTRFS snapshots)
- **Lifetime**: Configurable retention (e.g., 30 days)
- **Storage**: BTRFS snapshots + index

**Contents**:
- System configuration changes
- State transitions over time
- Problem-solution pairs
- Successful workflows

**Access Pattern**:
```rust
// Retrieve similar past situation
let similar_events = episodic_memory.search(SearchQuery {
    event_type: "service_restart",
    service: "nginx",
    timeframe: last_7_days,
});
```

### L4: Semantic Memory (Knowledge Base)
- **Scope**: System-wide knowledge
- **Size**: Unlimited (embedded resources)
- **Lifetime**: Permanent (compiled into binary)
- **Storage**: Embedded resources + external docs

**Contents**:
- D-Bus interface specifications
- System architecture documentation
- Best practices and patterns
- API references

**Access**:
```rust
// Retrieve relevant documentation
let docs = resource_registry.search("NetworkManager D-Bus API");
```

## Context Management Strategies

### Strategy 1: Sliding Window with Summarization

```python
def manage_context(conversation_history, max_tokens=4000):
    """Keep recent context + summarized older context"""

    recent = conversation_history[-5:]  # Last 5 messages
    older = conversation_history[:-5]

    # Summarize older context
    summary = summarize_key_facts(older)

    context = {
        "summary": summary,
        "recent": recent,
        "tokens": count_tokens(summary) + count_tokens(recent)
    }

    return context
```

### Strategy 2: Importance-Based Retention

```python
def select_context(messages, max_tokens=4000):
    """Keep most important messages within token budget"""

    # Score each message by importance
    scored = [(msg, importance_score(msg)) for msg in messages]
    scored.sort(key=lambda x: x[1], reverse=True)

    # Pack messages until token limit
    selected = []
    token_count = 0

    for msg, score in scored:
        msg_tokens = count_tokens(msg)
        if token_count + msg_tokens <= max_tokens:
            selected.append(msg)
            token_count += msg_tokens

    # Reorder chronologically
    selected.sort(key=lambda m: m.timestamp)
    return selected
```

### Strategy 3: Hierarchical Context Loading

```python
class HierarchicalContext:
    def __init__(self):
        self.layers = {
            "system": self.load_system_context(),    # Always included
            "session": self.load_session_context(),  # Current session
            "relevant": [],                           # Loaded on-demand
            "historical": []                          # Loaded if needed
        }

    def get_context(self, query, max_tokens=8000):
        """Load context layers based on need"""
        context = []
        remaining = max_tokens

        # Layer 1: System (always)
        context.append(self.layers["system"])
        remaining -= len(self.layers["system"])

        # Layer 2: Session (recent)
        context.append(self.layers["session"][:remaining//2])
        remaining -= len(self.layers["session"][:remaining//2])

        # Layer 3: Relevant (semantic search)
        relevant = self.search_relevant(query, limit=remaining//2)
        context.append(relevant)
        remaining -= len(relevant)

        # Layer 4: Historical (if space remains)
        if remaining > 100:
            historical = self.search_historical(query, limit=remaining)
            context.append(historical)

        return flatten(context)
```

## Knowledge Retrieval Patterns

### Pattern 1: Lazy Loading

```rust
impl ResourceRegistry {
    fn get_documentation(&self, topic: &str) -> Result<String> {
        // Check cache first
        if let Some(cached) = self.cache.get(topic) {
            return Ok(cached.clone());
        }

        // Load from embedded resources
        let doc = self.load_embedded(topic)?;

        // Cache for future use
        self.cache.insert(topic.to_string(), doc.clone());

        Ok(doc)
    }
}
```

### Pattern 2: Semantic Search with Caching

```rust
impl DbusIndex {
    fn semantic_search(&mut self, query: &str, top_k: usize) -> Vec<SearchResult> {
        // Check if we've seen this query recently
        if let Some(cached) = self.query_cache.get(query) {
            return cached.clone();
        }

        // Compute query embedding
        let query_vec = self.embed(query);

        // Search index
        let results = self.vector_index.search(query_vec, top_k);

        // Cache results
        self.query_cache.insert(query.to_string(), results.clone());

        results
    }
}
```

### Pattern 3: Hybrid Search (Keyword + Semantic)

```rust
fn hybrid_search(
    index: &DbusIndex,
    query: &str,
    top_k: usize,
) -> Vec<SearchResult> {
    // Keyword search (fast, exact matches)
    let keyword_results = index.search(query);

    // Semantic search (slower, fuzzy matches)
    let semantic_results = index.semantic_search(query, top_k * 2);

    // Merge and re-rank
    let combined = merge_results(keyword_results, semantic_results);
    let reranked = rerank_by_relevance(combined, query);

    reranked.into_iter().take(top_k).collect()
}
```

## State Persistence Patterns

### Pattern 1: Incremental Snapshots

```rust
impl SnapshotManager {
    fn create_incremental_snapshot(&self) -> Result<()> {
        let current_state = self.capture_state()?;
        let last_snapshot = self.load_latest_snapshot()?;

        // Only save the diff
        let diff = compute_diff(&last_snapshot, &current_state);

        if !diff.is_empty() {
            self.save_snapshot(Snapshot::Incremental {
                base: last_snapshot.id,
                diff,
                timestamp: Utc::now(),
            })?;
        }

        Ok(())
    }
}
```

### Pattern 2: Copy-on-Write Memory

```rust
struct CowMemory<T> {
    data: Arc<T>,
    modified: bool,
}

impl<T: Clone> CowMemory<T> {
    fn write(&mut self) -> &mut T {
        if !self.modified {
            // Make a copy on first write
            self.data = Arc::new((*self.data).clone());
            self.modified = true;
        }
        Arc::get_mut(&mut self.data).unwrap()
    }

    fn read(&self) -> &T {
        &self.data
    }
}
```

## Context Compression Techniques

### Technique 1: Fact Extraction

```python
def extract_facts(conversation):
    """Extract key facts from conversation"""
    facts = {
        "services": set(),
        "methods_called": [],
        "errors": [],
        "decisions": [],
    }

    for msg in conversation:
        # Extract entities
        if "service" in msg:
            facts["services"].add(msg["service"])

        # Track actions
        if "tool_call" in msg:
            facts["methods_called"].append(msg["tool_call"])

        # Note errors
        if "error" in msg:
            facts["errors"].append(msg["error"])

        # Record decisions
        if "decision" in msg:
            facts["decisions"].append(msg["decision"])

    return facts
```

### Technique 2: Temporal Binning

```python
def bin_by_time(events, bin_size="1h"):
    """Group events into time bins for compression"""
    bins = defaultdict(list)

    for event in events:
        bin_key = floor_to_bin(event.timestamp, bin_size)
        bins[bin_key].append(event)

    # Summarize each bin
    summaries = {}
    for bin_key, bin_events in bins.items():
        summaries[bin_key] = summarize_events(bin_events)

    return summaries
```

### Technique 3: Reference Compression

```rust
enum ContextItem {
    Inline(String),
    Reference { uri: String, excerpt: Option<String> },
}

fn compress_context(items: Vec<String>) -> Vec<ContextItem> {
    items.into_iter().map(|item| {
        if item.len() > 1000 {
            // Large items become references
            let uri = store_in_resource_db(&item);
            let excerpt = extract_summary(&item, 200);
            ContextItem::Reference { uri, excerpt: Some(excerpt) }
        } else {
            ContextItem::Inline(item)
        }
    }).collect()
}
```

## Adaptive Context Allocation

```rust
struct AdaptiveContextManager {
    total_budget: usize,
    allocations: HashMap<ContextLayer, usize>,
}

impl AdaptiveContextManager {
    fn allocate(&mut self, task_complexity: f32) {
        // Adjust allocation based on task
        if task_complexity > 0.8 {
            // Complex task: more relevant context
            self.allocations.insert(ContextLayer::System, 1000);
            self.allocations.insert(ContextLayer::Session, 2000);
            self.allocations.insert(ContextLayer::Relevant, 4000);
            self.allocations.insert(ContextLayer::Historical, 1000);
        } else {
            // Simple task: focus on recent context
            self.allocations.insert(ContextLayer::System, 500);
            self.allocations.insert(ContextLayer::Session, 6000);
            self.allocations.insert(ContextLayer::Relevant, 1000);
            self.allocations.insert(ContextLayer::Historical, 500);
        }
    }
}
```

## Memory Optimization Guidelines

1. **Prefer References Over Copies**: Store large data once, reference many times
2. **Lazy Load Documentation**: Only load resources when actually needed
3. **Cache Frequently Used**: Keep hot data in memory with LRU eviction
4. **Compress Incrementally**: Use diffs and deltas instead of full snapshots
5. **Index for Speed**: Pre-build indexes for common search patterns
6. **Expire Stale Data**: Remove context that's no longer relevant
7. **Summarize Aggressively**: Convert verbose logs to concise summaries
8. **Use Hierarchical Storage**: Hot (RAM) → Warm (Index) → Cold (Snapshot)
