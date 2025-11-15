use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Debian package metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub architecture: String,
    pub depends: Vec<String>,
    pub pre_depends: Vec<String>,
    pub recommends: Vec<String>,
    pub suggests: Vec<String>,
    pub conflicts: Vec<String>,
    pub replaces: Vec<String>,
    pub provides: Vec<String>,
    pub essential: bool,
    pub priority: String,
    pub section: String,
    pub description: String,
    pub filename: String,
    pub size: u64,
    pub md5sum: String,
    pub sha256: String,
}

impl Default for Package {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: String::new(),
            architecture: "amd64".to_string(),
            depends: Vec::new(),
            pre_depends: Vec::new(),
            recommends: Vec::new(),
            suggests: Vec::new(),
            conflicts: Vec::new(),
            replaces: Vec::new(),
            provides: Vec::new(),
            essential: false,
            priority: "optional".to_string(),
            section: String::new(),
            description: String::new(),
            filename: String::new(),
            size: 0,
            md5sum: String::new(),
            sha256: String::new(),
        }
    }
}

/// Installation stages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stages {
    pub essential: Vec<String>,
    pub required: Vec<String>,
    pub important: Vec<String>,
    pub standard: Vec<String>,
    pub proxmox: Vec<String>,
    pub optional: Vec<String>,
}

impl Default for Stages {
    fn default() -> Self {
        Self {
            essential: Vec::new(),
            required: Vec::new(),
            important: Vec::new(),
            standard: Vec::new(),
            proxmox: Vec::new(),
            optional: Vec::new(),
        }
    }
}

/// PackageKit installation manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub format: String,
    pub target: Target,
    pub metadata: Metadata,
    pub configuration: Configuration,
    pub stages: Vec<Stage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub distribution: String,
    pub version: String,
    pub architecture: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub total_packages: usize,
    pub stages: usize,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    pub default_batch_size: usize,
    pub default_retry_policy: String,
    pub max_retries: u32,
    pub retry_delay: u64,
    pub continue_on_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    pub name: String,
    pub description: String,
    pub priority: u32,
    pub package_count: usize,
    pub batch_size: usize,
    pub retry_policy: String,
    pub continue_on_error: bool,
    pub packages: Vec<PackageEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageEntry {
    pub name: String,
    pub version: String,
    pub architecture: String,
    pub essential: bool,
    pub depends: Vec<String>,
    pub pre_depends: Vec<String>,
}
