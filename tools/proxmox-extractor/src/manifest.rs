mod types;
mod parser;

use anyhow::{Context, Result};
use clap::Parser as ClapParser;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{info, warn};
use chrono::Utc;

use types::*;
use parser::parse_packages_file;

#[derive(ClapParser)]
#[command(name = "proxmox-manifest")]
#[command(about = "Generate PackageKit manifest from Proxmox package list")]
struct Args {
    /// Path to Packages file (e.g., Packages.txt from Proxmox ISO)
    #[arg(short, long)]
    packages: PathBuf,

    /// Output manifest path
    #[arg(short, long, default_value = "manifest.json")]
    output: PathBuf,

    /// Proxmox version
    #[arg(short, long, default_value = "9.0")]
    version: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    info!("Parsing packages from: {}", args.packages.display());
    let packages = parse_packages_file(&args.packages)
        .context("Failed to parse Packages file")?;

    info!("Found {} packages", packages.len());

    // Categorize into stages
    let stages = categorize_packages(&packages);

    info!("Categorized into stages:");
    info!("  Essential: {}", stages.essential.len());
    info!("  Required: {}", stages.required.len());
    info!("  Important: {}", stages.important.len());
    info!("  Standard: {}", stages.standard.len());
    info!("  Proxmox: {}", stages.proxmox.len());
    info!("  Optional: {}", stages.optional.len());

    // Generate manifest
    let manifest = generate_manifest(&packages, &stages, &args.version)?;

    // Write manifest
    let json = serde_json::to_string_pretty(&manifest)?;
    fs::write(&args.output, json)?;

    info!("Manifest written to: {}", args.output.display());

    Ok(())
}

fn categorize_packages(packages: &HashMap<String, Package>) -> Stages {
    let mut stages = Stages::default();

    for (name, pkg) in packages {
        if pkg.essential {
            stages.essential.push(name.clone());
        } else if pkg.priority == "required" {
            stages.required.push(name.clone());
        } else if pkg.priority == "important" {
            stages.important.push(name.clone());
        } else if pkg.priority == "standard" {
            stages.standard.push(name.clone());
        } else if pkg.section.starts_with("proxmox")
            || name.starts_with("pve-")
            || name.starts_with("proxmox-")
            || name.contains("pve")
        {
            stages.proxmox.push(name.clone());
        } else {
            stages.optional.push(name.clone());
        }
    }

    stages
}

fn generate_manifest(
    packages: &HashMap<String, Package>,
    stages: &Stages,
    version: &str,
) -> Result<Manifest> {
    let mut manifest_stages = Vec::new();

    // Stage configurations
    let stage_configs = [
        ("essential", &stages.essential, 10, "abort", false, 1),
        ("required", &stages.required, 20, "retry_transient", false, 2),
        ("important", &stages.important, 30, "retry_transient", false, 3),
        ("standard", &stages.standard, 50, "retry_transient", true, 4),
        ("proxmox", &stages.proxmox, 20, "retry_transient", false, 5),
        ("optional", &stages.optional, 50, "skip", true, 6),
    ];

    for (name, pkg_names, batch_size, retry_policy, continue_on_error, priority) in stage_configs {
        if pkg_names.is_empty() {
            continue;
        }

        let package_entries: Vec<PackageEntry> = pkg_names
            .iter()
            .filter_map(|pkg_name| {
                packages.get(pkg_name).map(|pkg| PackageEntry {
                    name: pkg.name.clone(),
                    version: pkg.version.clone(),
                    architecture: pkg.architecture.clone(),
                    essential: pkg.essential,
                    depends: pkg.depends.clone(),
                    pre_depends: pkg.pre_depends.clone(),
                })
            })
            .collect();

        manifest_stages.push(Stage {
            name: name.to_string(),
            description: format!("{} packages", name.replace('_', " ").to_uppercase()),
            priority,
            package_count: package_entries.len(),
            batch_size,
            retry_policy: retry_policy.to_string(),
            continue_on_error,
            packages: package_entries,
        });
    }

    Ok(Manifest {
        version: "1.0".to_string(),
        format: "proxmox-packagekit-manifest".to_string(),
        target: Target {
            distribution: "proxmox-ve".to_string(),
            version: version.to_string(),
            architecture: "amd64".to_string(),
        },
        metadata: Metadata {
            total_packages: packages.len(),
            stages: manifest_stages.len(),
            generated_at: Utc::now().to_rfc3339(),
        },
        configuration: Configuration {
            default_batch_size: 20,
            default_retry_policy: "retry_transient".to_string(),
            max_retries: 3,
            retry_delay: 5,
            continue_on_error: false,
        },
        stages: manifest_stages,
    })
}
