mod types;

use anyhow::{Context, Result};
use clap::Parser as ClapParser;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{info, warn, error};
use zbus::{Connection, proxy};

use types::*;

/// PackageKit D-Bus proxy
#[proxy(
    interface = "org.freedesktop.PackageKit",
    default_service = "org.freedesktop.PackageKit",
    default_path = "/org/freedesktop/PackageKit"
)]
trait PackageKit {
    /// Create a new transaction
    fn create_transaction(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    /// Get daemon state
    fn get_daemon_state(&self) -> zbus::Result<String>;
}

/// PackageKit Transaction proxy
#[proxy(
    interface = "org.freedesktop.PackageKit.Transaction",
    default_service = "org.freedesktop.PackageKit"
)]
trait Transaction {
    /// Install packages
    fn install_packages(&self, transaction_flags: u64, package_ids: Vec<String>) -> zbus::Result<()>;

    /// Refresh cache
    fn refresh_cache(&self, force: bool) -> zbus::Result<()>;

    /// Resolve package names
    fn resolve(&self, filters: u64, packages: Vec<String>) -> zbus::Result<()>;

    /// Get details for packages
    fn get_details(&self, package_ids: Vec<String>) -> zbus::Result<()>;

    // Signals
    #[zbus(signal)]
    fn finished(&self, exit_code: u32, runtime: u32) -> zbus::Result<()>;

    #[zbus(signal)]
    fn error_code(&self, code: u32, details: String) -> zbus::Result<()>;

    #[zbus(signal)]
    fn package(&self, info: u32, package_id: String, summary: String) -> zbus::Result<()>;
}

#[derive(ClapParser)]
#[command(name = "proxmox-packagekit")]
#[command(about = "Install Proxmox from manifest using PackageKit D-Bus")]
struct Args {
    /// Path to manifest.json
    manifest: PathBuf,

    /// Dry run - don't actually install packages
    #[arg(long)]
    dry_run: bool,

    /// Log file
    #[arg(short, long, default_value = "packagekit-install.log")]
    log_file: PathBuf,
}

struct PackageKitInstaller {
    connection: Connection,
    dry_run: bool,
    log_file: PathBuf,
    transaction_count: usize,
    total_packages_installed: usize,
    failed_packages: Vec<String>,
}

impl PackageKitInstaller {
    async fn new(dry_run: bool, log_file: PathBuf) -> Result<Self> {
        let connection = Connection::system().await?;

        Ok(Self {
            connection,
            dry_run,
            log_file,
            transaction_count: 0,
            total_packages_installed: 0,
            failed_packages: Vec::new(),
        })
    }

    fn log(&self, message: &str) {
        let timestamp = chrono::Local::now().format("%H:%M:%S");
        let log_line = format!("[{}] {}", timestamp, message);
        println!("{}", log_line);

        if let Ok(mut file) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)
        {
            use std::io::Write;
            let _ = writeln!(file, "{}", log_line);
        }
    }

    async fn refresh_cache(&mut self) -> Result<()> {
        self.log("━".repeat(80).as_str());
        self.log("Refreshing package cache...");
        self.log("━".repeat(80).as_str());

        if self.dry_run {
            self.log("[DRY RUN] Would refresh cache");
            return Ok(());
        }

        let pk_proxy = PackageKitProxy::new(&self.connection).await?;
        let transaction_path = pk_proxy.create_transaction().await?;

        let tx_proxy = TransactionProxy::builder(&self.connection)
            .path(transaction_path)?
            .build()
            .await?;

        tx_proxy.refresh_cache(true).await?;

        // Wait for completion
        sleep(Duration::from_secs(2)).await;

        self.log("✓ Cache refreshed successfully");
        Ok(())
    }

    async fn install_packages_batch(
        &mut self,
        packages: &[PackageEntry],
        batch_size: usize,
        retry_policy: &str,
    ) -> Result<(usize, usize)> {
        let mut success_count = 0;
        let mut failure_count = 0;

        for (i, chunk) in packages.chunks(batch_size).enumerate() {
            let batch_num = i + 1;
            let total_batches = (packages.len() + batch_size - 1) / batch_size;

            self.log("");
            self.log(&format!("📦 Batch {}/{} ({} packages)", batch_num, total_batches, chunk.len()));

            let pkg_names: Vec<String> = chunk.iter().map(|p| p.name.clone()).collect();
            self.log(&format!("   Packages: {}", pkg_names.join(", ")));

            match self.install_batch(&pkg_names).await {
                Ok(()) => {
                    success_count += chunk.len();
                    self.log("   ✓ Batch completed successfully");
                }
                Err(e) => {
                    self.log(&format!("   ✗ Batch failed: {}", e));

                    match retry_policy {
                        "abort" => {
                            self.log("   Policy: abort - stopping installation");
                            return Err(e);
                        }
                        "retry_transient" => {
                            self.log("   Policy: retry_transient - retrying individual packages");

                            for pkg in chunk {
                                match self.install_batch(&[pkg.name.clone()]).await {
                                    Ok(()) => {
                                        success_count += 1;
                                        self.log(&format!("      ✓ {}", pkg.name));
                                    }
                                    Err(_) => {
                                        failure_count += 1;
                                        self.failed_packages.push(pkg.name.clone());
                                        self.log(&format!("      ✗ {}", pkg.name));
                                    }
                                }
                            }
                        }
                        "skip" => {
                            self.log("   Policy: skip - continuing with next batch");
                            failure_count += chunk.len();
                            for pkg in chunk {
                                self.failed_packages.push(pkg.name.clone());
                            }
                        }
                        _ => {}
                    }
                }
            }

            self.transaction_count += 1;
        }

        Ok((success_count, failure_count))
    }

    async fn install_batch(&self, package_names: &[String]) -> Result<()> {
        if self.dry_run {
            self.log(&format!("[DRY RUN] Would install: {}", package_names.join(", ")));
            sleep(Duration::from_millis(100)).await;
            return Ok(());
        }

        let pk_proxy = PackageKitProxy::new(&self.connection).await?;
        let transaction_path = pk_proxy.create_transaction().await?;

        let tx_proxy = TransactionProxy::builder(&self.connection)
            .path(transaction_path)?
            .build()
            .await?;

        // For Proxmox packages, we need to use pkcon directly since PackageKit
        // may not properly resolve Proxmox repo packages via D-Bus
        // This is a workaround until we implement proper repo resolution

        // Use pkcon as fallback for actual installation
        let package_list = package_names.join(" ");
        let output = tokio::process::Command::new("pkcon")
            .arg("install")
            .arg("-y")
            .args(package_names)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("pkcon install failed: {}", stderr));
        }

        Ok(())
    }

    async fn process_stage(&mut self, stage: &Stage) -> Result<()> {
        self.log("");
        self.log(&"=".repeat(80));
        self.log(&format!("STAGE: {}", stage.name.to_uppercase()));
        self.log(&format!("Description: {}", stage.description));
        self.log(&format!("Packages: {}", stage.package_count));
        self.log(&format!("Batch size: {}", stage.batch_size));
        self.log(&format!("Retry policy: {}", stage.retry_policy));
        self.log(&"=".repeat(80));

        let start = Instant::now();

        let result = self
            .install_packages_batch(&stage.packages, stage.batch_size, &stage.retry_policy)
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok((success, failure)) => {
                self.log("");
                self.log(&format!("Stage '{}' completed:", stage.name));
                self.log(&format!("   ✓ Success: {}", success));
                self.log(&format!("   ✗ Failed: {}", failure));
                self.log(&format!("   ⏱️  Time: {:.1}s", elapsed.as_secs_f64()));

                if failure > 0 && !stage.continue_on_error {
                    return Err(anyhow::anyhow!("Stage failed with {} package failures", failure));
                }

                Ok(())
            }
            Err(e) => {
                if !stage.continue_on_error {
                    self.log(&format!("✗ Stage failed and continue_on_error=false: {}", e));
                    Err(e)
                } else {
                    self.log(&format!("⚠️  Stage had errors but continue_on_error=true: {}", e));
                    Ok(())
                }
            }
        }
    }

    async fn install_from_manifest(&mut self, manifest: &Manifest) -> Result<bool> {
        self.log("");
        self.log(&"=".repeat(80));
        self.log("STARTING INSTALLATION FROM MANIFEST");
        self.log(&format!("Target: {} {}", manifest.target.distribution, manifest.target.version));
        self.log(&format!("Total packages: {}", manifest.metadata.total_packages));
        self.log(&format!("Stages: {}", manifest.metadata.stages));
        self.log(&"=".repeat(80));

        // Refresh cache first
        if let Err(e) = self.refresh_cache().await {
            warn!("Failed to refresh cache: {}", e);
        }

        // Process each stage
        for stage in &manifest.stages {
            if let Err(e) = self.process_stage(stage).await {
                self.log(&format!("✗ Installation failed at stage '{}': {}", stage.name, e));
                return Ok(false);
            }
            self.total_packages_installed += stage.package_count;
        }

        // Final summary
        self.log("");
        self.log(&"=".repeat(80));
        self.log("INSTALLATION COMPLETE");
        self.log(&format!("Total transactions: {}", self.transaction_count));
        self.log(&format!("Packages processed: {}", self.total_packages_installed));
        self.log(&format!("Failed packages: {}", self.failed_packages.len()));
        if !self.failed_packages.is_empty() {
            self.log("");
            self.log("Failed packages:");
            for pkg in &self.failed_packages {
                self.log(&format!("   - {}", pkg));
            }
        }
        self.log(&"=".repeat(80));

        Ok(self.failed_packages.is_empty())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    if !args.manifest.exists() {
        error!("Manifest not found: {}", args.manifest.display());
        std::process::exit(1);
    }

    // Load manifest
    let manifest_json = fs::read_to_string(&args.manifest)?;
    let manifest: Manifest = serde_json::from_str(&manifest_json)?;

    info!("Loaded manifest: {} packages in {} stages",
        manifest.metadata.total_packages,
        manifest.metadata.stages
    );

    // Create installer
    let mut installer = PackageKitInstaller::new(args.dry_run, args.log_file).await?;

    // Run installation
    match installer.install_from_manifest(&manifest).await {
        Ok(true) => {
            info!("Installation completed successfully");
            Ok(())
        }
        Ok(false) => {
            error!("Installation completed with failures");
            std::process::exit(1);
        }
        Err(e) => {
            error!("Installation failed: {}", e);
            std::process::exit(1);
        }
    }
}
