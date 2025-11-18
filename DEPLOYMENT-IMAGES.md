# Deployment Images with BTRFS Snapshots

## Overview

The deployment image system creates deployment "images" as folders where:
- Each folder is a BTRFS snapshot for streaming deployment
- Files that exist in previous images are symlinked (automatic deduplication)
- New files are copied normally
- Images can be streamed for deployment

## Architecture

```
/var/lib/op-dbus/deployment/
├── images/
│   ├── PROXMOX-DBUS_STAGE/          # First image
│   │   ├── file1.txt                # Real file
│   │   ├── file2.txt                # Real file
│   │   └── .image-metadata.json     # Metadata
│   │
│   ├── PROXMOX-DBUS_STAGE-2/        # Second image
│   │   ├── file1.txt -> ../PROXMOX-DBUS_STAGE/file1.txt  # Symlink
│   │   ├── file3.txt                # New file
│   │   └── .image-metadata.json
│   │
│   └── PROXMOX-DBUS_STAGE-3/        # Third image
│       ├── file1.txt -> ../../PROXMOX-DBUS_STAGE/file1.txt  # Symlink
│       ├── file2.txt -> ../../PROXMOX-DBUS_STAGE/file2.txt  # Symlink
│       ├── file3.txt -> ../PROXMOX-DBUS_STAGE-2/file3.txt   # Symlink
│       ├── file4.txt                # New file
│       └── .image-metadata.json
│
└── snapshots/                        # BTRFS snapshots for streaming
    ├── PROXMOX-DBUS_STAGE-20250101-120000/
    ├── PROXMOX-DBUS_STAGE-2-20250101-130000/
    └── PROXMOX-DBUS_STAGE-3-20250101-140000/
```

## Usage

### Initialize Deployment Directory

```bash
sudo op-dbus image init
```

This creates the base directory structure at `/var/lib/op-dbus/deployment/`.

### Create a Deployment Image

```bash
# Create first image
sudo op-dbus image create PROXMOX-DBUS_STAGE \
  --files /path/to/file1.txt /path/to/file2.txt

# Create second image (file1.txt will be symlinked)
sudo op-dbus image create PROXMOX-DBUS_STAGE-2 \
  --files /path/to/file1.txt /path/to/file3.txt

# Create third image (file1.txt and file3.txt will be symlinked)
sudo op-dbus image create PROXMOX-DBUS_STAGE-3 \
  --files /path/to/file1.txt /path/to/file3.txt /path/to/file4.txt
```

### List Images

```bash
op-dbus image list
```

Output:
```
=== Deployment Images (3) ===

PROXMOX-DBUS_STAGE
  Created: 2025-01-01 12:00:00
  Path: /var/lib/op-dbus/deployment/images/PROXMOX-DBUS_STAGE
  Size: 2048 bytes (unique: 2048, symlinked: 0)
  Files: 2

PROXMOX-DBUS_STAGE-2
  Created: 2025-01-01 13:00:00
  Path: /var/lib/op-dbus/deployment/images/PROXMOX-DBUS_STAGE-2
  Size: 3072 bytes (unique: 1024, symlinked: 2048)
  Files: 2

PROXMOX-DBUS_STAGE-3
  Created: 2025-01-01 14:00:00
  Path: /var/lib/op-dbus/deployment/images/PROXMOX-DBUS_STAGE-3
  Size: 4096 bytes (unique: 1024, symlinked: 3072)
  Files: 3
```

### Show Image Details

```bash
op-dbus image show PROXMOX-DBUS_STAGE-2
```

### Get Streamable Snapshot Path

```bash
# Get the path to the snapshot for streaming
SNAPSHOT_PATH=$(op-dbus image snapshot PROXMOX-DBUS_STAGE-2)

# Stream the snapshot (example with btrfs send)
btrfs send $SNAPSHOT_PATH | ssh remote-host "btrfs receive /mnt/deployment/"
```

### Delete an Image

```bash
# Interactive deletion
sudo op-dbus image delete PROXMOX-DBUS_STAGE-2

# Force deletion (no confirmation)
sudo op-dbus image delete PROXMOX-DBUS_STAGE-2 --force
```

## How It Works

### Symlink Deduplication

When creating a new image:
1. The system checks all previous images (from newest to oldest)
2. If a file with the same name exists in a previous image:
   - A symlink is created pointing to the original file
   - The file size is counted as "symlinked" (not unique)
3. If the file doesn't exist in any previous image:
   - The file is copied to the new image
   - A SHA256 hash is calculated for deduplication
   - The file size is counted as "unique"

### BTRFS Snapshots

After creating an image:
1. A read-only BTRFS snapshot is created in `snapshots/`
2. The snapshot name includes the image name and timestamp
3. Snapshots can be streamed using `btrfs send` for deployment

### Benefits

1. **Space Efficient**: Overlapping files are symlinked, not duplicated
2. **Fast Deployment**: BTRFS snapshots can be streamed efficiently
3. **Version Control**: Each image is a complete snapshot
4. **Deduplication**: Automatic detection of overlapping files
5. **Metadata Tracking**: Each image has metadata about files and sizes

## Example Workflow

```bash
# 1. Initialize
sudo op-dbus image init

# 2. Create base image
sudo op-dbus image create PROXMOX-DBUS_STAGE \
  --files \
    target/release/op-dbus \
    /etc/op-dbus/state.json \
    systemd/op-dbus.service

# 3. Create updated image (only new/changed files)
sudo op-dbus image create PROXMOX-DBUS_STAGE-2 \
  --files \
    target/release/op-dbus \  # Will be symlinked if unchanged
    /etc/op-dbus/state.json \  # Will be symlinked if unchanged
    systemd/op-dbus.service \   # Will be symlinked if unchanged
    new-feature.so              # New file, will be copied

# 4. Get snapshot for streaming
SNAPSHOT=$(op-dbus image snapshot PROXMOX-DBUS_STAGE-2)

# 5. Stream to remote host
btrfs send $SNAPSHOT | ssh deploy-host "btrfs receive /mnt/deployment/"
```

## Notes

- Requires BTRFS filesystem for snapshots (works without BTRFS but snapshots disabled)
- Symlinks use relative paths for portability
- File deduplication is based on filename (not content hash)
- Snapshots are read-only and can be safely streamed
- Images are stored as BTRFS subvolumes when on BTRFS filesystem

