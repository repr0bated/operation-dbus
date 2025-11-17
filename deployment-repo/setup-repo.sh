#!/bin/bash
set -euo pipefail

# Operation D-Bus Deployment Repository Setup Script
# Initializes the deployment repository structure

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

make_scripts_executable() {
    log_step "Making scripts executable..."

    find "$SCRIPT_DIR" -name "*.sh" -type f -exec chmod +x {} \;

    log_info "Scripts made executable"
}

create_gitignore() {
    log_step "Creating .gitignore..."

    cat > "$SCRIPT_DIR/.gitignore" << 'EOF'
# Deployment artifacts
*.send
*.send.sha256
*.tar.gz

# Temporary files
/tmp/
/var/tmp/

# Logs
*.log

# Build artifacts
target/
Cargo.lock

# OS generated files
.DS_Store
.DS_Store?
._*
.Spotlight-V100
.Trashes
ehthumbs.db
Thumbs.db

# IDE files
.vscode/
.idea/
*.swp
*.swo

# Btrfs temporary mounts
/mnt/op-dbus-*
/tmp/op-dbus-*
EOF

    log_info ".gitignore created"
}

create_initial_commit_message() {
    log_step "Creating initial commit template..."

    cat > "$SCRIPT_DIR/.gitmessage" << 'EOF'
Operation D-Bus Deployment Update

- Updated deployment scripts
- Modified Btrfs snapshot handling
- Changed CI/CD pipeline

Related issues: #123
EOF

    log_info "Initial commit template created"
}

validate_structure() {
    log_step "Validating repository structure..."

    local required_files=(
        "README.md"
        "scripts/pre-deploy/validate-source.sh"
        "btrfs/scripts/create-snapshot.sh"
        "scripts/pre-deploy/prepare-target.sh"
        "scripts/post-deploy/mount-deployment.sh"
        "scripts/upgrade/rolling-upgrade.sh"
        "scripts/rollback/rollback-to.sh"
        ".github/workflows/build-snapshot.yml"
    )

    for file in "${required_files[@]}"; do
        if [[ ! -f "$SCRIPT_DIR/$file" ]]; then
            log_error "Required file missing: $file"
            exit 1
        fi
    done

    log_info "Repository structure validation passed"
}

create_repo_summary() {
    log_step "Creating repository summary..."

    cat > "$SCRIPT_DIR/REPO-SUMMARY.md" << 'EOF'
# Operation D-Bus Deployment Repository Summary

## Purpose
This repository manages Btrfs-based deployment of Operation D-Bus using snapshot send/receive operations for efficient, atomic system deployments and upgrades.

## Key Components

### Source Environment (Build System)
- Golden environment: Btrfs subvolume with complete op-dbus installation
- Snapshot creation: Automated Btrfs snapshot generation
- Send stream generation: Incremental or full send streams for distribution

### Target Environment (Deployment System)
- Btrfs receive: Atomic deployment of snapshots
- Overlay mounts: Writable overlay on read-only snapshots
- System integration: Symlinks and systemd service management

### Upgrade/Rollback
- Rolling upgrades: Zero-downtime version transitions
- Atomic rollback: Instant reversion to previous versions
- Incremental updates: Only transfer changed data

## Workflow

### Build System
1. Update golden environment with new op-dbus version
2. Create Btrfs snapshot: `./btrfs/scripts/create-snapshot.sh v1.2.3`
3. Generate send stream: `./btrfs/scripts/send-snapshot.sh v1.2.3`
4. Release via GitHub Actions

### Target System
1. Prepare system: `./scripts/pre-deploy/prepare-target.sh`
2. Receive snapshot: `sudo btrfs receive /var/lib/op-dbus/deploy < snapshot.send`
3. Mount deployment: `./scripts/post-deploy/mount-deployment.sh v1.2.3`
4. Upgrade: `./scripts/upgrade/rolling-upgrade.sh v1.2.3 v1.3.0`

## Security Model
- Read-only deployments prevent tampering
- SHA-256 checksum verification
- GPG signature verification (planned)
- Audit trail via blockchain (inherited from op-dbus)

## Performance Characteristics
- Initial deployment: Full send stream (size of installation)
- Incremental updates: Only changed data transferred
- Atomic operations: All-or-nothing deployments
- Overlay filesystem: Minimal write amplification

## Integration Points
- GitHub Actions: Automated snapshot creation and releases
- GitHub Container Registry: Docker images for testing
- GitHub Releases: Send streams and metadata
- Systemd: Service management and dependencies
- D-Bus: System integration (inherited)

## Maintenance
- Automatic cleanup: Old snapshots removed after 3 versions
- Validation: Pre/post-deployment checks
- Monitoring: Service health and version tracking
- Backup: Deployment records and overlay preservation
EOF

    log_info "Repository summary created"
}

show_next_steps() {
    log_info "Repository setup complete!"
    echo
    log_info "Next steps:"
    echo "1. Create GitHub repository:"
    echo "   https://github.com/new -> operation-dbus-deployment"
    echo
    echo "2. Push this repository:"
    echo "   cd deployment-repo"
    echo "   git init"
    echo "   git add ."
    echo "   git commit -m 'Initial deployment repository setup'"
    echo "   git remote add origin https://github.com/repr0bated/operation-dbus-deployment.git"
    echo "   git push -u origin main"
    echo
    echo "3. Set up build system:"
    echo "   - Configure self-hosted runner with Btrfs"
    echo "   - Set up golden environment subvolume"
    echo "   - Test snapshot creation workflow"
    echo
    echo "4. Configure CI/CD:"
    echo "   - Update workflow for your environment"
    echo "   - Set up GitHub Container Registry access"
    echo "   - Configure release notifications"
}

main() {
    log_info "Setting up Operation D-Bus deployment repository..."

    make_scripts_executable
    create_gitignore
    create_initial_commit_message
    validate_structure
    create_repo_summary

    show_next_steps
}

main "$@"