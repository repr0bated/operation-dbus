// ISP migration toolkit
// Analyzes current provider restrictions and recommends alternatives

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ISP analysis report for migration planning
#[derive(Debug, Serialize, Deserialize)]
pub struct IspMigrationReport {
    pub current_provider: ProviderAnalysis,
    pub detected_restrictions: Vec<Restriction>,
    pub recommended_providers: Vec<ProviderRecommendation>,
    pub migration_cost_analysis: CostAnalysis,
    pub migration_plan: MigrationPlan,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderAnalysis {
    pub name: String,
    pub detected_from: String, // "hostname", "network", "manual"
    pub service_type: ServiceType,
    pub restrictions_score: u32, // 0-100, higher = more restricted
    pub feature_availability: HashMap<String, FeatureAvailability>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ServiceType {
    SharedVPS,        // Multiple VMs on shared host
    DedicatedVPS,     // Dedicated resources, shared host
    DedicatedServer,  // Full physical server
    BareMetal,        // Dedicated server with full hardware access
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FeatureAvailability {
    Available,
    RestrictedByDefault,    // Available but must request
    RequiresUpgrade,        // Need to pay more
    NotOffered,            // Provider doesn't support
    TechnicallyBlocked,    // VM/container limitation
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Restriction {
    pub feature: String,
    pub severity: RestrictionSeverity,
    pub impact: String,
    pub workaround: Option<String>,
    pub unlockable: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RestrictionSeverity {
    Critical,   // Blocks primary use case
    High,       // Significant limitation
    Medium,     // Inconvenient but workable
    Low,        // Minor issue
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderRecommendation {
    pub name: String,
    pub url: String,
    pub service_type: ServiceType,
    pub starting_price_monthly: f64,
    pub gpu_passthrough: bool,
    pub nested_virt: bool,
    pub full_hardware_access: bool,
    pub iommu_available: bool,
    pub compatibility_score: u32, // 0-100
    pub pros: Vec<String>,
    pub cons: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CostAnalysis {
    pub current_monthly_cost: f64,
    pub feature_restrictions_opportunity_cost: f64, // What you lose due to restrictions
    pub recommended_provider_cost: f64,
    pub cost_difference: f64,
    pub roi_months: Option<f64>, // Payback period if switching
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub steps: Vec<MigrationStep>,
    pub estimated_downtime: String,
    pub automation_available: bool,
    pub data_export_commands: Vec<String>,
    pub data_import_commands: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationStep {
    pub order: u32,
    pub description: String,
    pub commands: Vec<String>,
    pub estimated_time: String,
    pub reversible: bool,
}

pub struct IspMigrationAnalyzer;

impl IspMigrationAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Analyze current ISP and generate migration report
    pub fn analyze(&self) -> Result<IspMigrationReport> {
        let current_provider = self.detect_current_provider()?;
        let restrictions = self.detect_restrictions()?;
        let recommended_providers = self.recommend_providers(&restrictions);
        let migration_cost = self.analyze_costs(&current_provider, &recommended_providers);
        let migration_plan = self.generate_migration_plan(&current_provider, &recommended_providers[0]);

        Ok(IspMigrationReport {
            current_provider,
            detected_restrictions: restrictions,
            recommended_providers,
            migration_cost_analysis: migration_cost,
            migration_plan,
        })
    }

    fn detect_current_provider(&self) -> Result<ProviderAnalysis> {
        // Detect provider from hostname, network info, etc.
        let hostname = hostname::get()?.to_string_lossy().to_string();

        let (name, service_type) = self.identify_provider(&hostname);

        let mut feature_availability = HashMap::new();

        // Check which features are available
        let is_vm = self.is_running_in_vm()?;

        if is_vm {
            // VPS/VM - many features restricted
            feature_availability.insert("gpu_passthrough".to_string(), FeatureAvailability::NotOffered);
            feature_availability.insert("nested_virt".to_string(), FeatureAvailability::RestrictedByDefault);
            feature_availability.insert("iommu".to_string(), FeatureAvailability::TechnicallyBlocked);
            feature_availability.insert("full_hardware_access".to_string(), FeatureAvailability::RequiresUpgrade);
        } else {
            // Dedicated/bare metal
            feature_availability.insert("gpu_passthrough".to_string(), FeatureAvailability::Available);
            feature_availability.insert("nested_virt".to_string(), FeatureAvailability::Available);
            feature_availability.insert("iommu".to_string(), FeatureAvailability::Available);
            feature_availability.insert("full_hardware_access".to_string(), FeatureAvailability::Available);
        }

        let restrictions_score = self.calculate_restriction_score(&feature_availability);

        Ok(ProviderAnalysis {
            name,
            detected_from: "hostname and virtualization detection".to_string(),
            service_type,
            restrictions_score,
            feature_availability,
        })
    }

    fn identify_provider(&self, hostname: &str) -> (String, ServiceType) {
        // Common provider patterns in hostnames
        if hostname.contains("ovh") || hostname.contains("kimsufi") {
            ("OVH/Kimsufi".to_string(), ServiceType::SharedVPS)
        } else if hostname.contains("hetzner") {
            ("Hetzner".to_string(), ServiceType::DedicatedServer)
        } else if hostname.contains("digitalocean") {
            ("DigitalOcean".to_string(), ServiceType::SharedVPS)
        } else if hostname.contains("linode") {
            ("Linode/Akamai".to_string(), ServiceType::SharedVPS)
        } else if hostname.contains("vultr") {
            ("Vultr".to_string(), ServiceType::SharedVPS)
        } else if hostname.contains("aws") || hostname.contains("amazon") {
            ("AWS EC2".to_string(), ServiceType::SharedVPS)
        } else if hostname.contains("gcp") || hostname.contains("google") {
            ("Google Cloud".to_string(), ServiceType::SharedVPS)
        } else if hostname.contains("azure") || hostname.contains("microsoft") {
            ("Azure".to_string(), ServiceType::SharedVPS)
        } else {
            ("Unknown Provider".to_string(), ServiceType::Unknown)
        }
    }

    fn is_running_in_vm(&self) -> Result<bool> {
        // Check systemd-detect-virt
        let output = std::process::Command::new("systemd-detect-virt")
            .output();

        if let Ok(out) = output {
            let virt_type = String::from_utf8_lossy(&out.stdout).trim().to_string();
            Ok(virt_type != "none")
        } else {
            // Fallback: check /proc/cpuinfo for hypervisor flag
            let cpuinfo = std::fs::read_to_string("/proc/cpuinfo")?;
            Ok(cpuinfo.contains("hypervisor"))
        }
    }

    fn calculate_restriction_score(&self, features: &HashMap<String, FeatureAvailability>) -> u32 {
        let mut score = 0u32;

        for availability in features.values() {
            score += match availability {
                FeatureAvailability::Available => 0,
                FeatureAvailability::RestrictedByDefault => 20,
                FeatureAvailability::RequiresUpgrade => 40,
                FeatureAvailability::NotOffered => 60,
                FeatureAvailability::TechnicallyBlocked => 80,
            };
        }

        score / features.len() as u32
    }

    fn detect_restrictions(&self) -> Result<Vec<Restriction>> {
        let mut restrictions = Vec::new();

        // Check GPU passthrough
        let has_gpu = std::path::Path::new("/dev/nvidia0").exists() ||
                     std::path::Path::new("/dev/dri/card0").exists();

        if !has_gpu {
            let lspci_output = std::process::Command::new("lspci")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .unwrap_or_default();

            let host_might_have_gpu = lspci_output.contains("VGA") ||
                                     lspci_output.contains("3D controller") ||
                                     lspci_output.contains("NVIDIA") ||
                                     lspci_output.contains("AMD/ATI");

            if !host_might_have_gpu || lspci_output.is_empty() {
                restrictions.push(Restriction {
                    feature: "GPU Passthrough".to_string(),
                    severity: RestrictionSeverity::Critical,
                    impact: "Cannot run ML workloads, video encoding, or GPU-accelerated applications. 100x slower processing.".to_string(),
                    workaround: Some("Use GPU cloud instances ($3-5/hour) or migrate to provider with GPU passthrough".to_string()),
                    unlockable: false, // Requires host access
                });
            }
        }

        // Check nested virtualization
        let cpuinfo = std::fs::read_to_string("/proc/cpuinfo")?;
        let has_vmx_or_svm = cpuinfo.contains("vmx") || cpuinfo.contains("svm");

        if !has_vmx_or_svm {
            let is_vm = self.is_running_in_vm()?;
            if is_vm {
                restrictions.push(Restriction {
                    feature: "Nested Virtualization".to_string(),
                    severity: RestrictionSeverity::High,
                    impact: "Cannot run Docker, Kubernetes, or other containerization inside VM. Blocks DevOps workflows.".to_string(),
                    workaround: Some("Use native containers on host, or migrate to provider with nested virt".to_string()),
                    unlockable: false,
                });
            }
        }

        // Check IOMMU
        let iommu_groups = std::path::Path::new("/sys/kernel/iommu_groups").exists();
        if !iommu_groups {
            restrictions.push(Restriction {
                feature: "IOMMU/VT-d".to_string(),
                severity: RestrictionSeverity::High,
                impact: "Cannot do PCI device passthrough (GPU, NVMe, network cards). No hardware isolation.".to_string(),
                workaround: None,
                unlockable: false,
            });
        }

        // Check CPU feature exposure
        let flags: Vec<&str> = cpuinfo
            .lines()
            .find(|l| l.starts_with("flags"))
            .unwrap_or("")
            .split_whitespace()
            .collect();

        // Typical bare-metal CPU has 80-120 flags
        // VPS often exposes only 40-60 flags
        if flags.len() < 60 {
            restrictions.push(Restriction {
                feature: "CPU Feature Exposure".to_string(),
                severity: RestrictionSeverity::Medium,
                impact: format!("Only {} CPU features exposed, expected 80-120. Some optimizations unavailable.", flags.len()),
                workaround: Some("Request host CPU passthrough from ISP".to_string()),
                unlockable: false,
            });
        }

        Ok(restrictions)
    }

    fn recommend_providers(&self, restrictions: &[Restriction]) -> Vec<ProviderRecommendation> {
        let mut providers = Vec::new();

        // Hetzner - Best for GPU + full control
        providers.push(ProviderRecommendation {
            name: "Hetzner Dedicated".to_string(),
            url: "https://www.hetzner.com/dedicated-rootserver".to_string(),
            service_type: ServiceType::BareMetal,
            starting_price_monthly: 39.0,
            gpu_passthrough: true,
            nested_virt: true,
            full_hardware_access: true,
            iommu_available: true,
            compatibility_score: 100,
            pros: vec![
                "Full IPMI access".to_string(),
                "No feature restrictions".to_string(),
                "GPU passthrough supported".to_string(),
                "Excellent price/performance".to_string(),
                "NixOS supported".to_string(),
            ],
            cons: vec![
                "Europe-only datacenters".to_string(),
                "GPU servers require auction".to_string(),
            ],
        });

        // OVH Dedicated
        providers.push(ProviderRecommendation {
            name: "OVH Dedicated Server".to_string(),
            url: "https://www.ovh.com/world/dedicated-servers/".to_string(),
            service_type: ServiceType::DedicatedServer,
            starting_price_monthly: 59.0,
            gpu_passthrough: true,
            nested_virt: true,
            full_hardware_access: true,
            iommu_available: true,
            compatibility_score: 95,
            pros: vec![
                "Worldwide datacenters".to_string(),
                "GPU options available".to_string(),
                "Full root access".to_string(),
                "IPMI/KVM access".to_string(),
            ],
            cons: vec![
                "More expensive than Hetzner".to_string(),
                "GPU passthrough requires manual setup".to_string(),
            ],
        });

        // Vultr Bare Metal
        providers.push(ProviderRecommendation {
            name: "Vultr Bare Metal".to_string(),
            url: "https://www.vultr.com/products/bare-metal/".to_string(),
            service_type: ServiceType::BareMetal,
            starting_price_monthly: 120.0,
            gpu_passthrough: true,
            nested_virt: true,
            full_hardware_access: true,
            iommu_available: true,
            compatibility_score: 90,
            pros: vec![
                "USA datacenters".to_string(),
                "Fast provisioning (minutes)".to_string(),
                "Full hardware access".to_string(),
            ],
            cons: vec![
                "Higher cost".to_string(),
                "Limited GPU options".to_string(),
            ],
        });

        // Scaleway Dedibox
        providers.push(ProviderRecommendation {
            name: "Scaleway Dedibox".to_string(),
            url: "https://www.scaleway.com/en/dedibox/".to_string(),
            service_type: ServiceType::DedicatedServer,
            starting_price_monthly: 15.99,
            gpu_passthrough: true,
            nested_virt: true,
            full_hardware_access: true,
            iommu_available: true,
            compatibility_score: 85,
            pros: vec![
                "Very affordable".to_string(),
                "Europe datacenters".to_string(),
                "Full access".to_string(),
            ],
            cons: vec![
                "Slower hardware (older servers)".to_string(),
                "No GPU options".to_string(),
                "Limited availability".to_string(),
            ],
        });

        // Sort by compatibility score
        providers.sort_by_key(|p| std::cmp::Reverse(p.compatibility_score));

        providers
    }

    fn analyze_costs(&self, current: &ProviderAnalysis, recommended: &[ProviderRecommendation]) -> CostAnalysis {
        // Estimate opportunity cost of restrictions
        let mut opportunity_cost = 0.0;

        // If GPU blocked, estimate cost of GPU cloud instances
        if matches!(
            current.feature_availability.get("gpu_passthrough"),
            Some(FeatureAvailability::NotOffered) | Some(FeatureAvailability::TechnicallyBlocked)
        ) {
            // Assume 100 hours/month of GPU compute needed
            opportunity_cost += 100.0 * 3.0; // $3/hour * 100 hours = $300/month
        }

        let current_monthly = 50.0; // Estimate, user should provide
        let recommended_cost = recommended.first().map(|p| p.starting_price_monthly).unwrap_or(100.0);
        let cost_diff = recommended_cost - current_monthly;

        let roi_months = if cost_diff < 0.0 || opportunity_cost > cost_diff.abs() {
            Some((cost_diff.abs() / (opportunity_cost - cost_diff)).max(0.0))
        } else {
            None
        };

        CostAnalysis {
            current_monthly_cost: current_monthly,
            feature_restrictions_opportunity_cost: opportunity_cost,
            recommended_provider_cost: recommended_cost,
            cost_difference: cost_diff,
            roi_months,
        }
    }

    fn generate_migration_plan(&self, _current: &ProviderAnalysis, target: &ProviderRecommendation) -> MigrationPlan {
        let mut steps = Vec::new();

        steps.push(MigrationStep {
            order: 1,
            description: "Export current system configuration with op-dbus".to_string(),
            commands: vec![
                "sudo op-dbus discover --export --generate-nix --output current-system.json".to_string(),
                "# This captures: hardware, kernel params, packages, services".to_string(),
            ],
            estimated_time: "5 minutes".to_string(),
            reversible: true,
        });

        steps.push(MigrationStep {
            order: 2,
            description: "Backup data and configurations".to_string(),
            commands: vec![
                "# Backup /etc, /home, /var/lib, application data".to_string(),
                "tar -czf etc-backup.tar.gz /etc".to_string(),
                "tar -czf home-backup.tar.gz /home".to_string(),
                "tar -czf varlib-backup.tar.gz /var/lib".to_string(),
            ],
            estimated_time: "30 minutes - 2 hours (depends on data size)".to_string(),
            reversible: true,
        });

        steps.push(MigrationStep {
            order: 3,
            description: format!("Provision new server at {}", target.name),
            commands: vec![
                format!("# Order server from: {}", target.url),
                "# Choose specs matching current-system.json requirements".to_string(),
                "# Request: GPU passthrough, IOMMU, nested virt if needed".to_string(),
            ],
            estimated_time: "1-24 hours (provisioning time)".to_string(),
            reversible: true,
        });

        steps.push(MigrationStep {
            order: 4,
            description: "Install NixOS with op-dbus on new server".to_string(),
            commands: vec![
                "# Boot into NixOS installer".to_string(),
                "# Copy current-system.nix to /mnt/etc/nixos/".to_string(),
                "nixos-install".to_string(),
                "reboot".to_string(),
            ],
            estimated_time: "30 minutes".to_string(),
            reversible: true,
        });

        steps.push(MigrationStep {
            order: 5,
            description: "Restore data to new server".to_string(),
            commands: vec![
                "# Transfer backups to new server".to_string(),
                "scp *-backup.tar.gz newserver:/root/".to_string(),
                "# Extract on new server".to_string(),
                "ssh newserver 'cd / && tar -xzf /root/etc-backup.tar.gz'".to_string(),
            ],
            estimated_time: "1-4 hours (depends on data size)".to_string(),
            reversible: true,
        });

        steps.push(MigrationStep {
            order: 6,
            description: "Verify op-dbus detects all features unlocked".to_string(),
            commands: vec![
                "ssh newserver 'sudo op-dbus discover'".to_string(),
                "# Verify: GPU passthrough available, nested virt enabled, IOMMU working".to_string(),
            ],
            estimated_time: "10 minutes".to_string(),
            reversible: false,
        });

        steps.push(MigrationStep {
            order: 7,
            description: "Update DNS and cutover traffic".to_string(),
            commands: vec![
                "# Update DNS A/AAAA records to point to new server IP".to_string(),
                "# Wait for DNS propagation (1-48 hours depending on TTL)".to_string(),
                "# Monitor old server for residual traffic".to_string(),
            ],
            estimated_time: "1-48 hours (DNS propagation)".to_string(),
            reversible: true,
        });

        steps.push(MigrationStep {
            order: 8,
            description: "Decommission old server (after verification period)".to_string(),
            commands: vec![
                "# Keep old server running for 1-2 weeks as fallback".to_string(),
                "# Cancel old ISP service".to_string(),
                "# Document lessons learned from migration".to_string(),
            ],
            estimated_time: "1-2 weeks overlap period".to_string(),
            reversible: false,
        });

        MigrationPlan {
            steps,
            estimated_downtime: "Near-zero (DNS cutover only)".to_string(),
            automation_available: true,
            data_export_commands: vec![
                "sudo op-dbus discover --export".to_string(),
                "tar -czf full-backup.tar.gz /etc /home /var/lib".to_string(),
            ],
            data_import_commands: vec![
                "scp current-system.nix newserver:/etc/nixos/".to_string(),
                "nixos-rebuild switch".to_string(),
            ],
        }
    }
}
