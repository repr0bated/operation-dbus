// Integration tests for D-Bus introspection cache
// Tests persistence, query performance, and data integrity

use anyhow::Result;
use std::path::PathBuf;
use tempfile::TempDir;

// Note: These tests require the op-dbus library to be built with the mcp feature
#[cfg(feature = "mcp")]
mod cache_tests {
    use super::*;
    use op_dbus::mcp::introspection_cache::IntrospectionCache;

    const SAMPLE_XML: &str = r#"<?xml version="1.0"?>
<!DOCTYPE node PUBLIC "-//freedesktop//DTD D-BUS Object Introspection 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd">
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
    <signal name="UnitNew">
      <arg name="id" type="s"/>
      <arg name="unit" type="o"/>
    </signal>
  </interface>
  <node name="unit"/>
</node>"#;

    #[test]
    fn test_cache_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let cache_path = temp_dir.path().join("test-cache.db");

        let cache = IntrospectionCache::new(&cache_path)?;

        // Cache file should exist
        assert!(cache_path.exists());

        Ok(())
    }

    #[test]
    fn test_store_and_retrieve() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let cache_path = temp_dir.path().join("test-cache.db");

        let cache = IntrospectionCache::new(&cache_path)?;

        // Store introspection data
        cache.store_introspection(
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            SAMPLE_XML,
        )?;

        // Retrieve full introspection JSON
        let result = cache.get_introspection_json(
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            Some("org.freedesktop.systemd1.Manager"),
        )?;

        assert!(result.is_some());
        let json = result.unwrap();

        // Verify interfaces exist
        assert!(json.get("interfaces").is_some());

        Ok(())
    }

    #[test]
    fn test_method_query() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let cache_path = temp_dir.path().join("test-cache.db");

        let cache = IntrospectionCache::new(&cache_path)?;

        // Store introspection data
        cache.store_introspection(
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            SAMPLE_XML,
        )?;

        // Query methods
        let methods = cache.get_methods_json(
            "org.freedesktop.systemd1",
            "org.freedesktop.systemd1.Manager",
        )?;

        // Should have 2 methods: StartUnit and StopUnit
        assert!(methods.is_array());
        let methods_array = methods.as_array().unwrap();
        assert_eq!(methods_array.len(), 2);

        Ok(())
    }

    #[test]
    fn test_method_search() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let cache_path = temp_dir.path().join("test-cache.db");

        let cache = IntrospectionCache::new(&cache_path)?;

        // Store introspection data
        cache.store_introspection(
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            SAMPLE_XML,
        )?;

        // Search for methods containing "Start"
        let results = cache.search_methods("%Start%")?;

        assert!(!results.is_empty());
        assert_eq!(results.len(), 1); // Only StartUnit

        Ok(())
    }

    #[test]
    fn test_cache_persistence() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let cache_path = temp_dir.path().join("test-cache.db");

        // First session: store data
        {
            let cache = IntrospectionCache::new(&cache_path)?;
            cache.store_introspection(
                "org.freedesktop.systemd1",
                "/org/freedesktop/systemd1",
                SAMPLE_XML,
            )?;
        }

        // Second session: verify data persists
        {
            let cache = IntrospectionCache::new(&cache_path)?;
            let result = cache.get_introspection_json(
                "org.freedesktop.systemd1",
                "/org/freedesktop/systemd1",
                Some("org.freedesktop.systemd1.Manager"),
            )?;

            assert!(result.is_some());
        }

        Ok(())
    }

    #[test]
    fn test_cache_update() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let cache_path = temp_dir.path().join("test-cache.db");

        let cache = IntrospectionCache::new(&cache_path)?;

        // Store initial data
        cache.store_introspection(
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            SAMPLE_XML,
        )?;

        // Update with new data (same service/path)
        let updated_xml = SAMPLE_XML.replace("StartUnit", "StartUnitUpdated");
        cache.store_introspection(
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            &updated_xml,
        )?;

        // Verify updated data
        let methods = cache.get_methods_json(
            "org.freedesktop.systemd1",
            "org.freedesktop.systemd1.Manager",
        )?;

        let methods_str = serde_json::to_string(&methods)?;
        assert!(methods_str.contains("StartUnitUpdated"));

        Ok(())
    }

    #[test]
    fn test_cache_stats() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let cache_path = temp_dir.path().join("test-cache.db");

        let cache = IntrospectionCache::new(&cache_path)?;

        // Store some data
        cache.store_introspection(
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            SAMPLE_XML,
        )?;

        // Get stats
        let stats = cache.get_stats()?;

        assert!(stats.get("total_services").is_some());
        assert!(stats.get("total_methods").is_some());

        Ok(())
    }

    #[test]
    fn test_old_cache_cleanup() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let cache_path = temp_dir.path().join("test-cache.db");

        let cache = IntrospectionCache::new(&cache_path)?;

        // Store introspection data
        cache.store_introspection(
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            SAMPLE_XML,
        )?;

        // Clear entries older than 0 days (should clear all)
        let deleted = cache.clear_old_cache(0)?;

        assert!(deleted > 0);

        // Verify cache is empty
        let result = cache.get_introspection_json(
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            Some("org.freedesktop.systemd1.Manager"),
        )?;

        assert!(result.is_none());

        Ok(())
    }

    #[test]
    fn test_multiple_services() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let cache_path = temp_dir.path().join("test-cache.db");

        let cache = IntrospectionCache::new(&cache_path)?;

        // Store multiple services
        cache.store_introspection(
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            SAMPLE_XML,
        )?;

        cache.store_introspection(
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            SAMPLE_XML, // Using same XML for simplicity
        )?;

        // Verify both services are cached
        let stats = cache.get_stats()?;
        let total_services = stats.get("total_services")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        assert!(total_services >= 2);

        Ok(())
    }

    #[test]
    fn test_cache_performance() -> Result<()> {
        use std::time::Instant;

        let temp_dir = TempDir::new()?;
        let cache_path = temp_dir.path().join("test-cache.db");

        let cache = IntrospectionCache::new(&cache_path)?;

        // Store data
        cache.store_introspection(
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            SAMPLE_XML,
        )?;

        // Measure retrieval time
        let start = Instant::now();
        for _ in 0..100 {
            let _ = cache.get_methods_json(
                "org.freedesktop.systemd1",
                "org.freedesktop.systemd1.Manager",
            )?;
        }
        let duration = start.elapsed();

        // 100 queries should complete in under 100ms (1ms per query average)
        assert!(duration.as_millis() < 100,
            "Cache queries too slow: {:?}", duration);

        println!("Cache performance: {} queries in {:?}", 100, duration);

        Ok(())
    }
}
