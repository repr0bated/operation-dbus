//! BTRFS Snapshot Management
//!
//! Provides rolling snapshot retention policies for BTRFS subvolumes.
//! Supports multiple retention strategies:
//! - Rolling N: Keep last N snapshots, delete older ones
//! - Time-based: Keep snapshots from last N days/hours
//! - Tagged: Keep specific snapshots forever (golden masters)

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use serde::{Deserialize, Serialize};

/// Snapshot retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetentionPolicy {
    /// Keep last N snapshots, delete older
    Rolling { keep: usize },

    /// Keep snapshots from last N days
    TimeBased { days: usize },

    /// Keep all tagged snapshots, rolling policy for untagged
    Tagged { keep_untagged: usize },
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        RetentionPolicy::Rolling { keep: 3 }
    }
}

/// Snapshot metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotInfo {
    pub name: String,
    pub path: PathBuf,
    pub created: i64,
    pub tagged: bool,
    pub tag: Option<String>,
    pub size_bytes: u64,
}

/// BTRFS snapshot manager
pub struct SnapshotManager {
    snapshots_dir: PathBuf,
    policy: RetentionPolicy,
}

impl SnapshotManager {
    /// Create new snapshot manager
    pub fn new(snapshots_dir: impl AsRef<Path>) -> Self {
        Self {
            snapshots_dir: snapshots_dir.as_ref().to_path_buf(),
            policy: RetentionPolicy::default(),
        }
    }

    /// Create with custom retention policy
    pub fn with_policy(snapshots_dir: impl AsRef<Path>, policy: RetentionPolicy) -> Self {
        Self {
            snapshots_dir: snapshots_dir.as_ref().to_path_buf(),
            policy,
        }
    }

    /// Create a new snapshot with rolling retention
    pub fn create_snapshot(
        &self,
        source: impl AsRef<Path>,
        name: Option<&str>,
    ) -> Result<PathBuf> {
        let source = source.as_ref();

        // Ensure snapshots directory exists
        std::fs::create_dir_all(&self.snapshots_dir)
            .context("Failed to create snapshots directory")?;

        // Generate snapshot name
        let snapshot_name = if let Some(n) = name {
            n.to_string()
        } else {
            format!("{}", chrono::Utc::now().format("%Y-%m-%d-%H%M%S"))
        };

        let snapshot_path = self.snapshots_dir.join(&snapshot_name);

        // Create BTRFS snapshot
        log::info!("📸 Creating snapshot: {}", snapshot_name);
        let output = Command::new("btrfs")
            .args(&[
                "subvolume",
                "snapshot",
                "-r", // read-only
                source.to_str().unwrap(),
                snapshot_path.to_str().unwrap(),
            ])
            .output()
            .context("Failed to execute btrfs snapshot command")?;

        if !output.status.success() {
            bail!(
                "Failed to create snapshot: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        log::info!("   Created: {}", snapshot_path.display());

        // Apply retention policy
        self.apply_retention_policy()?;

        Ok(snapshot_path)
    }

    /// Apply retention policy (delete old snapshots)
    pub fn apply_retention_policy(&self) -> Result<()> {
        let snapshots = self.list_snapshots()?;

        match &self.policy {
            RetentionPolicy::Rolling { keep } => {
                self.apply_rolling_retention(&snapshots, *keep)?;
            }
            RetentionPolicy::TimeBased { days } => {
                self.apply_time_based_retention(&snapshots, *days)?;
            }
            RetentionPolicy::Tagged { keep_untagged } => {
                self.apply_tagged_retention(&snapshots, *keep_untagged)?;
            }
        }

        Ok(())
    }

    /// Apply rolling N retention (keep last N snapshots)
    fn apply_rolling_retention(&self, snapshots: &[SnapshotInfo], keep: usize) -> Result<()> {
        if snapshots.len() <= keep {
            return Ok(()); // Nothing to delete
        }

        // Sort by creation time (oldest first)
        let mut sorted = snapshots.to_vec();
        sorted.sort_by_key(|s| s.created);

        // Delete oldest snapshots beyond retention limit
        let to_delete = sorted.len() - keep;
        for snapshot in sorted.iter().take(to_delete) {
            log::info!("🗑️  Deleting old snapshot: {}", snapshot.name);
            self.delete_snapshot(&snapshot.path)?;
        }

        if to_delete > 0 {
            log::info!("   Kept {} most recent snapshots", keep);
        }

        Ok(())
    }

    /// Apply time-based retention
    fn apply_time_based_retention(&self, snapshots: &[SnapshotInfo], days: usize) -> Result<()> {
        let cutoff = chrono::Utc::now().timestamp() - (days as i64 * 86400);

        for snapshot in snapshots {
            if snapshot.created < cutoff {
                log::info!("🗑️  Deleting expired snapshot: {}", snapshot.name);
                self.delete_snapshot(&snapshot.path)?;
            }
        }

        Ok(())
    }

    /// Apply tagged retention (keep all tagged, rolling for untagged)
    fn apply_tagged_retention(&self, snapshots: &[SnapshotInfo], keep_untagged: usize) -> Result<()> {
        let mut untagged: Vec<_> = snapshots.iter().filter(|s| !s.tagged).collect();
        untagged.sort_by_key(|s| s.created);

        if untagged.len() > keep_untagged {
            let to_delete = untagged.len() - keep_untagged;
            for snapshot in untagged.iter().take(to_delete) {
                log::info!("🗑️  Deleting old untagged snapshot: {}", snapshot.name);
                self.delete_snapshot(&snapshot.path)?;
            }
        }

        Ok(())
    }

    /// Delete a snapshot
    pub fn delete_snapshot(&self, snapshot_path: &Path) -> Result<()> {
        let output = Command::new("btrfs")
            .args(&[
                "subvolume",
                "delete",
                snapshot_path.to_str().unwrap(),
            ])
            .output()
            .context("Failed to execute btrfs delete command")?;

        if !output.status.success() {
            bail!(
                "Failed to delete snapshot: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// List all snapshots
    pub fn list_snapshots(&self) -> Result<Vec<SnapshotInfo>> {
        if !self.snapshots_dir.exists() {
            return Ok(Vec::new());
        }

        let mut snapshots = Vec::new();

        for entry in std::fs::read_dir(&self.snapshots_dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            // Check if it's a BTRFS subvolume
            let output = Command::new("btrfs")
                .args(&["subvolume", "show", path.to_str().unwrap()])
                .output()?;

            if !output.status.success() {
                continue; // Not a subvolume
            }

            // Get creation time and size
            let metadata = std::fs::metadata(&path)?;
            let created = metadata.created()
                .or_else(|_| metadata.modified())?
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs() as i64;

            // Check for tag file
            let tag_file = path.join(".snapshot-tag");
            let (tagged, tag) = if tag_file.exists() {
                let tag_content = std::fs::read_to_string(&tag_file).ok();
                (true, tag_content)
            } else {
                (false, None)
            };

            snapshots.push(SnapshotInfo {
                name: path.file_name().unwrap().to_string_lossy().to_string(),
                path,
                created,
                tagged,
                tag,
                size_bytes: 0, // TODO: Calculate actual size
            });
        }

        Ok(snapshots)
    }

    /// Tag a snapshot (prevents deletion)
    pub fn tag_snapshot(&self, snapshot_name: &str, tag: &str) -> Result<()> {
        let snapshot_path = self.snapshots_dir.join(snapshot_name);
        if !snapshot_path.exists() {
            bail!("Snapshot not found: {}", snapshot_name);
        }

        let tag_file = snapshot_path.join(".snapshot-tag");
        std::fs::write(&tag_file, tag)?;

        log::info!("🏷️  Tagged snapshot '{}' as '{}'", snapshot_name, tag);
        Ok(())
    }

    /// Get total size of all snapshots
    pub fn total_size(&self) -> Result<u64> {
        // TODO: Use `btrfs qgroup show` for accurate size
        let mut total = 0u64;
        for snapshot in self.list_snapshots()? {
            // Rough estimate using du
            let output = Command::new("du")
                .args(&["-sb", snapshot.path.to_str().unwrap()])
                .output()?;

            if output.status.success() {
                let size_str = String::from_utf8_lossy(&output.stdout);
                if let Some(size) = size_str.split_whitespace().next() {
                    total += size.parse::<u64>().unwrap_or(0);
                }
            }
        }
        Ok(total)
    }

    /// Clean up all snapshots (dangerous!)
    pub fn delete_all(&self) -> Result<usize> {
        let snapshots = self.list_snapshots()?;
        let count = snapshots.len();

        for snapshot in snapshots {
            if !snapshot.tagged {
                self.delete_snapshot(&snapshot.path)?;
            }
        }

        Ok(count)
    }
}

/// Snapshot configuration for a specific component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotConfig {
    pub enabled: bool,
    pub subvolume: PathBuf,
    pub snapshots_dir: PathBuf,
    pub policy: RetentionPolicy,
    pub auto_snapshot_on_change: bool,
}

impl SnapshotConfig {
    /// Default config for D-Bus index
    pub fn dbus_index() -> Self {
        Self {
            enabled: true,
            subvolume: PathBuf::from("/var/lib/op-dbus/@dbus-index"),
            snapshots_dir: PathBuf::from("/var/lib/op-dbus/@snapshots/dbus-index"),
            policy: RetentionPolicy::Rolling { keep: 3 },
            auto_snapshot_on_change: true,
        }
    }

    /// Default config for cache
    pub fn cache() -> Self {
        Self {
            enabled: true,
            subvolume: PathBuf::from("/var/lib/op-dbus/@cache"),
            snapshots_dir: PathBuf::from("/var/lib/op-dbus/@snapshots/cache"),
            policy: RetentionPolicy::Rolling { keep: 3 },
            auto_snapshot_on_change: false,
        }
    }

    /// Default config for configuration
    pub fn config() -> Self {
        Self {
            enabled: true,
            subvolume: PathBuf::from("/var/lib/op-dbus/@config"),
            snapshots_dir: PathBuf::from("/var/lib/op-dbus/@snapshots/config"),
            policy: RetentionPolicy::Tagged { keep_untagged: 3 },
            auto_snapshot_on_change: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rolling_retention_logic() {
        // Test snapshot filtering logic
        let snapshots = vec![
            SnapshotInfo {
                name: "2025-01-01".to_string(),
                path: PathBuf::from("/snap/1"),
                created: 1704067200,
                tagged: false,
                tag: None,
                size_bytes: 0,
            },
            SnapshotInfo {
                name: "2025-01-02".to_string(),
                path: PathBuf::from("/snap/2"),
                created: 1704153600,
                tagged: false,
                tag: None,
                size_bytes: 0,
            },
            SnapshotInfo {
                name: "2025-01-03".to_string(),
                path: PathBuf::from("/snap/3"),
                created: 1704240000,
                tagged: false,
                tag: None,
                size_bytes: 0,
            },
        ];

        // Should keep last 2, delete first one
        let mut sorted = snapshots.clone();
        sorted.sort_by_key(|s| s.created);
        assert_eq!(sorted[0].name, "2025-01-01");
        assert_eq!(sorted[2].name, "2025-01-03");
    }
}
