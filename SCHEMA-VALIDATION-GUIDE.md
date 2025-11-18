# Schema Validation Guide

**Prevents random/unrealistic schema generation from Proxmox templates**

## Problem

When generating schemas from a Proxmox template with 5,444,000 elements, it can create random use cases from 4,400 possibilities, many of which are unrealistic or invalid.

## Solution

The `SchemaValidator` provides:
1. **Curated Use Case Templates** - Pre-defined realistic use cases
2. **Dependency Validation** - Ensures required plugins are present
3. **Field Constraint Validation** - Validates field values and combinations
4. **Schema Suggestions** - Generates realistic schemas based on use cases

## Usage

### Validate a Generated Schema

```rust
use op_dbus::state::schema_validator::SchemaValidator;

let validator = SchemaValidator::new();
let schema: Value = serde_json::from_str(&generated_json)?;

let result = validator.validate_schema(&schema)?;

if !result.valid {
    eprintln!("Schema validation failed:");
    for error in &result.errors {
        eprintln!("  ❌ {}", error);
    }
}

for warning in &result.warnings {
    eprintln!("  ⚠️  {}", warning);
}

if let Some(use_case) = &result.matched_use_case {
    println!("✓ Matched use case: {}", use_case);
}
```

### Get Curated Use Cases

```rust
let validator = SchemaValidator::new();

for use_case in validator.get_use_cases() {
    println!("Use case: {} - {}", use_case.name, use_case.description);
    println!("  Required plugins: {:?}", use_case.required_plugins);
}
```

### Suggest Realistic Schema

```rust
// Instead of random generation, use curated use case
let schema = validator.suggest_schema("privacy_router")
    .expect("Use case not found");

println!("Suggested schema: {}", serde_json::to_string_pretty(&schema)?);
```

## Curated Use Cases

### 1. Privacy Router
- **Required Plugins**: `privacy_router`, `openflow`, `net`, `lxc`
- **Description**: Multi-hop privacy tunnel with WireGuard, WARP, and XRay
- **Valid Combinations**:
  - WireGuard container ID: 100
  - XRay container ID: 101
  - Bridge name: `ovsbr0` or `vmbr0`

### 2. Basic Network
- **Required Plugins**: `net`
- **Description**: Basic OVS bridge with DHCP

### 3. Container Mesh
- **Required Plugins**: `lxc`, `netmaker`, `openflow`
- **Description**: LXC containers with Netmaker mesh networking

## Adding New Use Cases

Edit `src/state/schema_validator.rs` and add to `load_default_use_cases()`:

```rust
UseCaseTemplate {
    name: "my_use_case".to_string(),
    description: "Description of use case".to_string(),
    required_plugins: vec!["plugin1".to_string(), "plugin2".to_string()],
    required_fields: {
        let mut m = HashMap::new();
        m.insert("plugin1".to_string(), vec!["field1".to_string()]);
        m
    },
    valid_combinations: vec![],
    dependencies: vec![],
    constraints: vec![],
},
```

## Integration with Schema Generation

Instead of generating random schemas, use:

1. **Use case list**: "Generate a schema for one of these use cases: privacy_router, basic_network, container_mesh"
2. **Validation feedback**: Validate generated schema and provide errors/warnings
3. **Suggestion fallback**: If validation fails, suggest a realistic schema

## Example: Constrained Generation

```python
# Instead of: "Generate a random schema from 5M elements"
# Use: "Generate a schema matching the 'privacy_router' use case template"

validator = SchemaValidator()
use_cases = [uc.name for uc in validator.get_use_cases()]

prompt = f"""
Generate a schema for one of these use cases: {', '.join(use_cases)}

Requirements:
- Must include all required plugins for the chosen use case
- Must use valid field combinations
- Must satisfy all dependencies

After generation, validate the schema and provide feedback.
"""
```

## Benefits

1. **Realistic Schemas**: Only generates valid, tested configurations
2. **Faster Iteration**: No need to debug invalid combinations
3. **Better Documentation**: Use cases serve as examples
4. **Maintainability**: Centralized validation logic

