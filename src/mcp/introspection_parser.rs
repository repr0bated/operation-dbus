// D-Bus XML Introspection → JSON Converter
// Parses D-Bus XML and converts to clean JSON structure

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrospectionData {
    pub interfaces: Vec<InterfaceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceInfo {
    pub name: String,
    pub methods: Vec<MethodInfo>,
    pub properties: Vec<PropertyInfo>,
    pub signals: Vec<SignalInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodInfo {
    pub name: String,
    pub inputs: Vec<ArgInfo>,
    pub outputs: Vec<ArgInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyInfo {
    pub name: String,
    pub type_sig: String,
    pub access: String, // "read", "write", "readwrite"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalInfo {
    pub name: String,
    pub args: Vec<ArgInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgInfo {
    pub name: String,
    pub type_sig: String, // D-Bus type signature (s=string, i=int, etc)
    pub type_name: String, // Friendly name (string, int, etc)
}

pub struct IntrospectionParser;

impl IntrospectionParser {
    /// Parse D-Bus XML introspection → JSON structure
    pub fn parse_xml(xml: &str) -> IntrospectionData {
        let mut interfaces = Vec::new();

        // Simple line-by-line parser (would use xml-rs or quick-xml in production)
        let mut current_interface: Option<InterfaceInfo> = None;
        let mut current_method: Option<MethodInfo> = None;

        for line in xml.lines() {
            let trimmed = line.trim();

            // Interface start
            if trimmed.starts_with("<interface name=") {
                if let Some(name) = Self::extract_attr(trimmed, "name") {
                    // Skip standard D-Bus interfaces
                    if !name.starts_with("org.freedesktop.DBus") {
                        current_interface = Some(InterfaceInfo {
                            name,
                            methods: Vec::new(),
                            properties: Vec::new(),
                            signals: Vec::new(),
                        });
                    }
                }
            }

            // Interface end
            if trimmed.starts_with("</interface>") {
                if let Some(iface) = current_interface.take() {
                    interfaces.push(iface);
                }
            }

            // Method start
            if trimmed.starts_with("<method name=") {
                if let Some(name) = Self::extract_attr(trimmed, "name") {
                    current_method = Some(MethodInfo {
                        name,
                        inputs: Vec::new(),
                        outputs: Vec::new(),
                    });
                }
            }

            // Method end
            if trimmed.starts_with("</method>") {
                if let (Some(ref mut iface), Some(method)) = (&mut current_interface, current_method.take()) {
                    iface.methods.push(method);
                }
            }

            // Method argument
            if trimmed.starts_with("<arg ") {
                if let (Some(ref mut method), Some(name), Some(type_sig), Some(direction)) = (
                    &mut current_method,
                    Self::extract_attr(trimmed, "name"),
                    Self::extract_attr(trimmed, "type"),
                    Self::extract_attr(trimmed, "direction"),
                ) {
                    let arg = ArgInfo {
                        name,
                        type_sig: type_sig.clone(),
                        type_name: Self::dbus_type_to_friendly(&type_sig),
                    };

                    if direction == "in" {
                        method.inputs.push(arg);
                    } else {
                        method.outputs.push(arg);
                    }
                }
            }

            // Property
            if trimmed.starts_with("<property name=") {
                if let (Some(ref mut iface), Some(name), Some(type_sig), Some(access)) = (
                    &mut current_interface,
                    Self::extract_attr(trimmed, "name"),
                    Self::extract_attr(trimmed, "type"),
                    Self::extract_attr(trimmed, "access"),
                ) {
                    iface.properties.push(PropertyInfo {
                        name,
                        type_sig,
                        access,
                    });
                }
            }

            // Signal
            if trimmed.starts_with("<signal name=") {
                if let (Some(ref mut iface), Some(name)) = (&mut current_interface, Self::extract_attr(trimmed, "name")) {
                    iface.signals.push(SignalInfo {
                        name,
                        args: Vec::new(),
                    });
                }
            }
        }

        IntrospectionData { interfaces }
    }

    /// Convert to clean JSON string
    pub fn to_json(data: &IntrospectionData) -> String {
        serde_json::to_string_pretty(data).unwrap()
    }

    /// Extract XML attribute value
    fn extract_attr(line: &str, attr: &str) -> Option<String> {
        let pattern = format!("{}=\"", attr);
        if let Some(start) = line.find(&pattern) {
            let start = start + pattern.len();
            if let Some(end) = line[start..].find('"') {
                return Some(line[start..start + end].to_string());
            }
        }
        None
    }

    /// Convert D-Bus type signature to friendly name
    fn dbus_type_to_friendly(sig: &str) -> String {
        match sig {
            "s" => "string".to_string(),
            "i" => "int32".to_string(),
            "u" => "uint32".to_string(),
            "x" => "int64".to_string(),
            "t" => "uint64".to_string(),
            "d" => "double".to_string(),
            "b" => "boolean".to_string(),
            "o" => "object_path".to_string(),
            "as" => "array<string>".to_string(),
            "a{ss}" => "dict<string,string>".to_string(),
            _ => sig.to_string(),
        }
    }

    /// Convert D-Bus type to MCP JSON schema type
    pub fn dbus_type_to_mcp_schema(sig: &str) -> serde_json::Value {
        use serde_json::json;

        match sig {
            // Basic types
            "s" => json!({"type": "string", "description": "String value"}),
            "o" => json!({"type": "string", "description": "D-Bus object path"}),
            "g" => json!({"type": "string", "description": "D-Bus signature"}),
            "y" => json!({"type": "integer", "minimum": 0, "maximum": 255, "description": "Byte (0-255)"}),
            "b" => json!({"type": "boolean", "description": "Boolean value"}),
            "n" => json!({"type": "integer", "minimum": -32768, "maximum": 32767}),
            "q" => json!({"type": "integer", "minimum": 0, "maximum": 65535}),
            "i" => json!({"type": "integer", "description": "32-bit signed integer"}),
            "u" => json!({"type": "integer", "minimum": 0, "description": "32-bit unsigned integer"}),
            "x" => json!({"type": "integer", "description": "64-bit signed integer"}),
            "t" => json!({"type": "integer", "minimum": 0, "description": "64-bit unsigned integer"}),
            "d" => json!({"type": "number", "description": "Double precision float"}),
            "h" => json!({"type": "integer", "description": "File descriptor handle"}),
            
            // Array types
            "as" => json!({
                "type": "array",
                "items": {"type": "string"},
                "description": "Array of strings"
            }),
            "ai" => json!({
                "type": "array",
                "items": {"type": "integer"},
                "description": "Array of integers"
            }),
            "ao" => json!({
                "type": "array",
                "items": {"type": "string"},
                "description": "Array of object paths"
            }),
            "ay" => json!({
                "type": "string",
                "description": "Byte array (base64 encoded)"
            }),
            "au" => json!({
                "type": "array",
                "items": {"type": "integer", "minimum": 0},
                "description": "Array of unsigned integers"
            }),
            
            // Dictionary types
            "a{sv}" => json!({
                "type": "object",
                "additionalProperties": true,
                "description": "Dictionary with string keys and variant values"
            }),
            "a{ss}" => json!({
                "type": "object",
                "additionalProperties": {"type": "string"},
                "description": "Dictionary with string keys and string values"
            }),
            "a{sa{sv}}" => json!({
                "type": "object",
                "additionalProperties": {
                    "type": "object",
                    "additionalProperties": true
                },
                "description": "Nested dictionary structure"
            }),
            
            // Tuple/struct types
            "(ss)" => json!({
                "type": "array",
                "items": {"type": "string"},
                "minItems": 2,
                "maxItems": 2,
                "description": "Tuple of two strings"
            }),
            
            // Complex or unknown types default to structured input
            _ if sig.starts_with("a{") => json!({
                "type": "object",
                "additionalProperties": true,
                "description": format!("Dictionary type: {}", sig)
            }),
            _ if sig.starts_with("a") => json!({
                "type": "array",
                "description": format!("Array type: {}", sig)
            }),
            _ if sig.starts_with("(") => json!({
                "type": "array",
                "description": format!("Tuple/struct type: {}", sig)
            }),
            
            // Fallback
            _ => json!({
                "type": "string",
                "description": format!("D-Bus type: {} (provide as JSON string)", sig)
            })
        }
    }
}

// Example usage demo
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_example() {
        let xml = r#"
<node>
  <interface name="org.freedesktop.systemd1.Manager">
    <method name="StartUnit">
      <arg name="name" type="s" direction="in"/>
      <arg name="mode" type="s" direction="in"/>
      <arg name="job" type="o" direction="out"/>
    </method>
    <method name="StopUnit">
      <arg name="name" type="s" direction="in"/>
      <arg name="mode" type="s" direction="in"/>
      <arg name="job" type="o" direction="out"/>
    </method>
    <property name="Version" type="s" access="read"/>
  </interface>
</node>
        "#;

        let data = IntrospectionParser::parse_xml(xml);
        let json = IntrospectionParser::to_json(&data);

        println!("JSON output:\n{}", json);

        assert_eq!(data.interfaces.len(), 1);
        assert_eq!(data.interfaces[0].methods.len(), 2);
        assert_eq!(data.interfaces[0].properties.len(), 1);
    }
}

// CLI tool to convert XML → JSON
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        println!("Usage: introspection-parser <service-name>");
        println!("   or: introspection-parser --stdin");
        println!("\nConverts D-Bus XML introspection to JSON");
        return Ok(());
    }

    let xml = if args[1] == "--stdin" {
        // Read from stdin
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        buffer
    } else {
        // Fetch from D-Bus service using busctl
        let service = &args[1];
        let output = std::process::Command::new("busctl")
            .arg("--user")
            .arg("introspect")
            .arg("--xml-interface")
            .arg(service)
            .arg("/")
            .output()?;

        if !output.status.success() {
            eprintln!("Error: Failed to introspect service {}", service);
            std::process::exit(1);
        }

        String::from_utf8_lossy(&output.stdout).to_string()
    };

    // Parse XML → JSON
    let data = IntrospectionParser::parse_xml(&xml);
    let json = IntrospectionParser::to_json(&data);

    println!("{}", json);

    Ok(())
}
