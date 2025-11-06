# Windows Support for op-dbus

## Executive Summary

**Goal**: Unified infrastructure management for mixed Windows/Linux environments

**Approaches** (in order of maturity):
1. **Hybrid Management**: NixOS control plane manages Windows via WinRM/SSH
2. **WSL2 Integration**: Run op-dbus natively in Windows via WSL2
3. **Native Windows Agent**: Port op-dbus to Windows with D-Bus support
4. **Nix on Windows**: Deploy Windows using Nix package manager (community project)

## Why This Matters

### The Enterprise Reality

**Typical enterprise infrastructure**:
- **70-80%** Windows (laptops, workstations, some servers)
- **20-30%** Linux (servers, containers, infrastructure)
- **<5%** macOS (executives, design teams)

**Current tools are siloed**:
- SCCM/Intune for Windows
- Ansible/Chef/Puppet for Linux
- Jamf for macOS

**op-dbus opportunity**: One tool for all platforms

### Market Size

If op-dbus can manage Windows:
- **Total addressable market**: $20B+ (vs $5B Linux-only)
- **Competitive advantage**: No other tool unifies Nix + Windows + blockchain audit
- **Enterprise adoption**: Removes "Linux-only" objection

---

## Approach 1: Hybrid Management (Available Today)

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                  NixOS Control Plane                    │
│                                                         │
│  ┌──────────────────────────────────────────────────┐  │
│  │              op-dbus Master                       │  │
│  │  - Declarative state (configuration.nix)         │  │
│  │  - Blockchain audit trail                        │  │
│  │  - BTRFS snapshots                               │  │
│  └──────────────────────────────────────────────────┘  │
│                        │                                │
│         ┌──────────────┼──────────────┐                │
│         │              │               │                │
└─────────┼──────────────┼───────────────┼────────────────┘
          │              │               │
    ┌─────▼──────┐ ┌────▼─────┐  ┌─────▼──────┐
    │   Linux    │ │  Linux   │  │  Windows   │
    │   Server   │ │  Laptop  │  │  Laptop    │
    │            │ │          │  │            │
    │ (native)   │ │(native)  │  │ (WinRM)    │
    └────────────┘ └──────────┘  └────────────┘
```

### Implementation

**NixOS configuration** (`configuration.nix`):
```nix
{ config, pkgs, ... }:

{
  services.op-dbus = {
    enable = true;

    # Define ALL infrastructure (Linux + Windows)
    state = {
      version = 1;

      # Linux machines (native)
      plugins = {
        systemd = {
          units = {
            "nginx.service" = {
              active_state = "active";
              enabled = true;
            };
          };
        };
      };

      # Windows machines (via remote management)
      windows = {
        hosts = [
          {
            hostname = "win-laptop-01.corp.internal";
            management = "winrm";  # or "ssh" if OpenSSH installed

            services = {
              "Winmgmt" = {  # Windows Management Instrumentation
                startup_type = "automatic";
                state = "running";
              };
              "wuauserv" = {  # Windows Update
                startup_type = "automatic";
                state = "running";
              };
              "Spooler" = {  # Print Spooler (disable if no printer)
                startup_type = "disabled";
                state = "stopped";
              };
            };

            features = {
              "TelnetClient" = "disabled";
              "SMB1Protocol" = "disabled";  # Security
            };

            registry = {
              # Disable Windows telemetry
              "HKLM\\SOFTWARE\\Policies\\Microsoft\\Windows\\DataCollection" = {
                "AllowTelemetry" = 0;
              };

              # Enforce strong passwords
              "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Lsa" = {
                "LimitBlankPasswordUse" = 1;
              };
            };

            packages = [
              # Chocolatey packages
              "googlechrome"
              "vscode"
              "7zip"
              "git"
            ];
          }
        ];
      };
    };
  };

  # Enable WinRM client for remote management
  environment.systemPackages = with pkgs; [
    winexe      # Execute commands on Windows
    freerdp     # RDP client (for troubleshooting)
  ];
}
```

### Windows Plugin Implementation

**File**: `src/plugins/windows.rs`

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowsHost {
    pub hostname: String,
    pub management: WindowsManagement,
    pub services: HashMap<String, WindowsService>,
    pub features: HashMap<String, String>,
    pub registry: HashMap<String, HashMap<String, serde_json::Value>>,
    pub packages: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WindowsManagement {
    WinRM,  // Windows Remote Management
    SSH,    // OpenSSH (Windows 10+)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowsService {
    pub startup_type: String,  // automatic, manual, disabled
    pub state: String,          // running, stopped
}

pub struct WindowsPlugin {
    hosts: Vec<WindowsHost>,
}

impl WindowsPlugin {
    pub async fn apply(&self) -> Result<()> {
        for host in &self.hosts {
            info!("Managing Windows host: {}", host.hostname);

            // Manage services
            for (service_name, desired_state) in &host.services {
                self.manage_service(&host, service_name, desired_state).await?;
            }

            // Manage Windows features
            for (feature_name, desired_state) in &host.features {
                self.manage_feature(&host, feature_name, desired_state).await?;
            }

            // Manage registry
            for (key_path, values) in &host.registry {
                self.manage_registry(&host, key_path, values).await?;
            }

            // Manage packages (via Chocolatey)
            self.manage_packages(&host).await?;
        }

        Ok(())
    }

    async fn manage_service(
        &self,
        host: &WindowsHost,
        service_name: &str,
        desired: &WindowsService,
    ) -> Result<()> {
        let ps_script = format!(
            r#"
            $service = Get-Service -Name '{}'

            # Set startup type
            Set-Service -Name '{}' -StartupType {}

            # Set state
            if ('{}' -eq 'running' -and $service.Status -ne 'Running') {{
                Start-Service -Name '{}'
            }} elseif ('{}' -eq 'stopped' -and $service.Status -ne 'Stopped') {{
                Stop-Service -Name '{}'
            }}
            "#,
            service_name,
            service_name, desired.startup_type,
            desired.state,
            service_name,
            desired.state,
            service_name,
        );

        self.exec_powershell(host, &ps_script).await?;
        Ok(())
    }

    async fn manage_registry(
        &self,
        host: &WindowsHost,
        key_path: &str,
        values: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        for (value_name, value) in values {
            let ps_script = match value {
                serde_json::Value::Number(n) => {
                    format!(
                        "Set-ItemProperty -Path '{}' -Name '{}' -Value {} -Type DWord",
                        key_path, value_name, n
                    )
                }
                serde_json::Value::String(s) => {
                    format!(
                        "Set-ItemProperty -Path '{}' -Name '{}' -Value '{}' -Type String",
                        key_path, value_name, s
                    )
                }
                _ => continue,
            };

            self.exec_powershell(host, &ps_script).await?;
        }

        Ok(())
    }

    async fn manage_packages(&self, host: &WindowsHost) -> Result<()> {
        // Ensure Chocolatey is installed
        let install_choco = r#"
        if (!(Test-Path "$env:ProgramData\chocolatey\choco.exe")) {
            Set-ExecutionPolicy Bypass -Scope Process -Force
            [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072
            iex ((New-Object System.Net.WebClient).DownloadString('https://chocolatey.org/install.ps1'))
        }
        "#;

        self.exec_powershell(host, install_choco).await?;

        // Install packages
        for package in &host.packages {
            let install_cmd = format!("choco install {} -y", package);
            self.exec_powershell(host, &install_cmd).await?;
        }

        Ok(())
    }

    async fn exec_powershell(&self, host: &WindowsHost, script: &str) -> Result<String> {
        match host.management {
            WindowsManagement::WinRM => {
                // Use winrm crate or winexe
                let output = Command::new("winexe")
                    .arg(format!("//{}", host.hostname))
                    .arg("powershell")
                    .arg("-Command")
                    .arg(script)
                    .output()
                    .context("Failed to execute winexe")?;

                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            }
            WindowsManagement::SSH => {
                // Use SSH (OpenSSH on Windows 10+)
                let output = Command::new("ssh")
                    .arg(&host.hostname)
                    .arg("powershell")
                    .arg("-Command")
                    .arg(script)
                    .output()
                    .context("Failed to execute SSH")?;

                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            }
        }
    }
}
```

### Advantages

✅ **Works today**: No Windows-side installation needed
✅ **Unified config**: One `configuration.nix` for all platforms
✅ **Blockchain audit**: Windows changes tracked just like Linux
✅ **Leverage existing tools**: WinRM, PowerShell, Chocolatey

### Limitations

⚠ **Requires WinRM/SSH**: Must be enabled on Windows machines
⚠ **Network dependency**: Can't manage offline Windows machines
⚠ **PowerShell only**: No native binary execution

---

## Approach 2: WSL2 Integration (Available Today)

### Architecture

Run op-dbus **inside** Windows via WSL2:

```
┌──────────────────────────────────────────┐
│          Windows 10/11 Laptop            │
│                                          │
│  ┌────────────────────────────────────┐  │
│  │          WSL2 (NixOS)              │  │
│  │                                    │  │
│  │  ┌──────────────────────────────┐  │  │
│  │  │      op-dbus Agent           │  │  │
│  │  │                              │  │  │
│  │  │  - Manage Linux (WSL)        │  │  │
│  │  │  - Manage Windows (WinRM)    │◄─┼──┼── Localhost
│  │  └──────────────────────────────┘  │  │
│  │                                    │  │
│  └────────────────────────────────────┘  │
│                                          │
│  Windows Services (managed via WinRM)    │
│  - Firewall                              │
│  - Windows Update                        │
│  - Antivirus                             │
└──────────────────────────────────────────┘
```

### Implementation

**Install NixOS in WSL2**:
```powershell
# Install WSL2
wsl --install

# Install NixOS (community WSL image)
wsl --import NixOS $env:LOCALAPPDATA\NixOS\ nixos-wsl.tar.gz

# Enter NixOS
wsl -d NixOS
```

**Configure op-dbus in WSL2**:
```nix
{ config, pkgs, ... }:

{
  services.op-dbus = {
    enable = true;

    state = {
      version = 1;

      # Manage Windows host (localhost via WinRM)
      windows = {
        hosts = [{
          hostname = "localhost";
          management = "winrm";

          services = {
            "wuauserv" = {
              startup_type = "automatic";
              state = "running";
            };
          };
        }];
      };

      # Manage WSL2 Linux environment
      plugins = {
        systemd = {
          units = {
            "docker.service" = {
              active_state = "active";
              enabled = true;
            };
          };
        };
      };
    };
  };
}
```

### Advantages

✅ **Self-contained**: op-dbus + NixOS run on Windows
✅ **Unified management**: Manage both WSL2 and Windows from one config
✅ **Developer-friendly**: Engineers can test locally
✅ **No separate infrastructure**: Each laptop is self-managed

### Limitations

⚠ **WSL2 required**: Windows 10 2004+ or Windows 11
⚠ **Performance overhead**: Extra layer (WSL2 → Windows)
⚠ **Complexity**: Users need to understand WSL2

---

## Approach 3: Native Windows Agent (Future)

### Architecture

Port op-dbus to run natively on Windows:

```
┌──────────────────────────────────────────┐
│          Windows 10/11 Laptop            │
│                                          │
│  ┌────────────────────────────────────┐  │
│  │    op-dbus.exe (Native Binary)     │  │
│  │                                    │  │
│  │  - Rust binary (cross-compiled)    │  │
│  │  - D-Bus for Windows               │  │
│  │  - PowerShell DSC backend          │  │
│  │  - Registry management             │  │
│  │  - Windows Services control        │  │
│  └────────────────────────────────────┘  │
│                                          │
│  Managed by op-dbus:                     │
│  - Windows Services                      │
│  - Registry keys                         │
│  - Windows Features                      │
│  - Installed applications                │
│  - Firewall rules                        │
└──────────────────────────────────────────┘
```

### Technology Stack

1. **D-Bus for Windows**: https://gitlab.freedesktop.org/dbus/dbus/-/tree/master/dbus
   - Maintained by freedesktop.org
   - Works on Windows natively
   - Used by Telegram, VLC, other apps

2. **PowerShell DSC**: Desired State Configuration
   - Native Windows configuration management
   - Declarative syntax (similar to Nix)
   - Idempotent operations

3. **Rust cross-compilation**: `cargo build --target x86_64-pc-windows-gnu`
   - Single binary
   - No runtime dependencies
   - Fast and efficient

### Example Configuration

```nix
{ config, pkgs, ... }:

{
  # Central NixOS server defines Windows policy
  services.op-dbus = {
    enable = true;

    # Windows machines pull config from here
    windows-fleet = {
      domain = "corp.internal";

      # Global policy for all Windows machines
      global-policy = {
        services = {
          "wuauserv" = { state = "running"; startup = "automatic"; };
          "Spooler" = { state = "stopped"; startup = "disabled"; };
        };

        registry = {
          "HKLM\\SOFTWARE\\Policies\\Microsoft\\Windows\\DataCollection" = {
            "AllowTelemetry" = 0;
          };
        };

        firewall = {
          enabled = true;
          default-inbound = "block";
          default-outbound = "allow";
        };
      };

      # Per-host overrides
      hosts = [
        {
          hostname = "exec-laptop-01";
          profile = "executive";  # High security

          services = {
            "Bluetooth Support" = { state = "stopped"; startup = "disabled"; };
          };
        }
        {
          hostname = "dev-laptop-42";
          profile = "developer";

          services = {
            "docker" = { state = "running"; startup = "automatic"; };
          };
        }
      ];
    };
  };
}
```

**Windows agent** (`C:\Program Files\op-dbus\op-dbus-agent.exe`):
- Runs as Windows Service
- Polls NixOS server for config (or subscribes via D-Bus)
- Applies state using PowerShell DSC
- Reports status back to blockchain

### Advantages

✅ **Native performance**: No WSL2 overhead
✅ **Offline support**: Agent runs locally, syncs when online
✅ **Mature protocol**: D-Bus proven on Windows
✅ **Leverage DSC**: PowerShell DSC is battle-tested

### Implementation Roadmap

**Phase 1**: Proof of concept (2-3 weeks)
- [ ] Port core op-dbus to Windows
- [ ] Integrate D-Bus for Windows
- [ ] Implement Windows Service management
- [ ] Test on Windows 10/11

**Phase 2**: Feature parity (1-2 months)
- [ ] Registry management
- [ ] Windows Features
- [ ] Chocolatey integration
- [ ] Firewall rules
- [ ] Group Policy alternatives

**Phase 3**: Production readiness (1-2 months)
- [ ] Windows Service installer
- [ ] Auto-update mechanism
- [ ] Central management server
- [ ] Comprehensive testing

**Phase 4**: Enterprise features (2-3 months)
- [ ] Active Directory integration
- [ ] Certificate-based auth
- [ ] Encrypted communication
- [ ] Compliance reporting

---

## Approach 4: Nix on Windows (Experimental)

### Overview

The Nix community has experimental Windows support:
- https://github.com/NixOS/nix/issues/5298
- https://github.com/nix-windows/nix

**Vision**: Deploy Windows using Nix packages
```nix
{ config, pkgs, ... }:

{
  environment.systemPackages = with pkgs.windows; [
    firefox
    chrome
    vscode
    office365
  ];
}
```

### Status

⚠ **Experimental**: Not production-ready
⏳ **Timeline**: 1-2 years until stable
🎯 **Potential**: True unified Nix experience

---

## Recommendation: Hybrid Approach (Phase 1) + Native Agent (Phase 2)

### Phase 1: Ship Today (Approach 1)
- Implement Windows plugin for remote management
- Use WinRM/SSH to manage Windows from NixOS control plane
- Get customer feedback
- **Timeline**: 2-4 weeks

### Phase 2: Native Experience (Approach 3)
- Port op-dbus to Windows
- Build native agent
- Integrate D-Bus for Windows
- **Timeline**: 3-6 months

### Phase 3: Full Parity
- Feature parity with Linux management
- Active Directory integration
- Enterprise-grade Windows support
- **Timeline**: 6-12 months

---

## Market Impact

### With Windows Support

**Before**: "Nice Linux tool, but we're mostly Windows"
**After**: "Finally, one tool for our entire fleet"

**TAM increases**:
- Linux-only: $5-10B
- Linux + Windows: $20B+
- **4x larger market**

**Competitive advantage**:
- SCCM: Windows-only
- Ansible: No blockchain audit
- Puppet/Chef: Expensive, no blockchain
- **op-dbus**: Only tool with Nix + Windows + blockchain

---

## Summary

**Can op-dbus deploy Windows machines?**

**Short answer**: Yes, three ways:
1. ✅ **Today**: Remote management via WinRM/SSH (Approach 1)
2. ✅ **Today**: WSL2 integration (Approach 2)
3. 🚧 **Soon**: Native Windows agent (Approach 3)

**Recommendation**: Start with Approach 1 (remote management) while building Approach 3 (native agent).

**Timeline**:
- **Month 1**: Windows plugin via WinRM (basic)
- **Month 2-3**: Enhanced Windows management (registry, features, packages)
- **Month 4-6**: Native Windows agent (production-ready)
- **Month 7-12**: Feature parity with Linux

**This positions op-dbus as the first truly unified infrastructure management platform.**
