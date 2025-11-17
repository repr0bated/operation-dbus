# Operation-DBus Master Documentation Index

> **Single source of truth for all documentation**

## 📖 Quick Start Guide

**New to the project?** Start here:
1. Read [README.md](../README.md) in project root
2. Follow [QUICKSTART.md](../QUICKSTART.md)
3. Install with [install.sh](../install.sh)

## 🗂️ Documentation Organization

### Getting Started
Essential docs for new users.

| Document | Description | Audience |
|----------|-------------|----------|
| [README.md](../README.md) | Main project overview | Everyone |
| [QUICKSTART.md](../QUICKSTART.md) | Quick reference | New users |
| [INSTALL-COMPLETE.md](../INSTALL-COMPLETE.md) | Installation guide | Admins |

### Architecture & Design
Deep technical documentation.

| Document | Description | Audience |
|----------|-------------|----------|
| [ARCHITECTURE-ANALYSIS.md](../ARCHITECTURE-ANALYSIS.md) | Blockchain/vector analysis | Architects |
| [MCP-ARCHITECTURE-ANALYSIS.md](../MCP-ARCHITECTURE-ANALYSIS.md) | MCP server analysis | Architects |
| [arch-review.md](../arch-review.md) | Overall architecture | Architects |
| [CLI-DESIGN.md](../CLI-DESIGN.md) | CLI patterns | Developers |

### MCP Integration
Model Context Protocol server and AI integration.

| Document | Description | Audience |
|----------|-------------|----------|
| [MCP-README.md](../MCP-README.md) | MCP overview | Everyone |
| [MCP-INTEGRATION.md](../MCP-INTEGRATION.md) | Integration guide | Developers |
| [MCP-DBUS-BENEFITS.md](../MCP-DBUS-BENEFITS.md) | Benefits analysis | Everyone |
| [MCP-COMPLETE-GUIDE.md](MCP-COMPLETE-GUIDE.md) | Complete guide | Developers |
| [MCP-API-REFERENCE.md](MCP-API-REFERENCE.md) | API reference | Developers |
| [MCP-DEVELOPER-GUIDE.md](MCP-DEVELOPER-GUIDE.md) | Developer guide | Developers |

### Deployment
Production deployment guides.

| Document | Description | Audience |
|----------|-------------|----------|
| [DEPLOYMENT.md](../DEPLOYMENT.md) | Basic deployment | DevOps |
| [ENTERPRISE-DEPLOYMENT.md](../ENTERPRISE-DEPLOYMENT.md) | Enterprise setup | DevOps |
| [CONTAINER-SETUP.md](../CONTAINER-SETUP.md) | Container configuration | Admins |

### Networking
Network configuration and architecture.

| Document | Description | Audience |
|----------|-------------|----------|
| [DUAL-BRIDGE-ARCHITECTURE.md](../DUAL-BRIDGE-ARCHITECTURE.md) | Bridge architecture | Network engineers |
| [NETMAKER-MESH-FIX.md](../NETMAKER-MESH-FIX.md) | Netmaker mesh fix | Admins |
| [CONTAINER-NETMAKER-SETUP.md](../CONTAINER-NETMAKER-SETUP.md) | Netmaker containers | Admins |
| [NETWORKMANAGER-PLUGIN-DESIGN.md](../NETWORKMANAGER-PLUGIN-DESIGN.md) | NetworkManager plugin | Developers |

### Caching & Storage
BTRFS and caching strategies.

| Document | Description | Audience |
|----------|-------------|----------|
| [INTROSPECTION-JSON-CACHE.md](INTROSPECTION-JSON-CACHE.md) | D-Bus introspection SQLite cache | Developers |
| [NONNET-INTROSPECTION-INTEGRATION.md](NONNET-INTROSPECTION-INTEGRATION.md) | Introspection DB integration options | Architects |
| [OVSDB-INTROSPECTION-SCHEMA.md](OVSDB-INTROSPECTION-SCHEMA.md) | OVSDB-based introspection approach | Architects |
| [BTRFS-CACHING-STRATEGY.md](../BTRFS-CACHING-STRATEGY.md) | BTRFS caching | Developers |
| [CACHING-IMPLEMENTED.md](../CACHING-IMPLEMENTED.md) | Implementation status | Developers |
| [BTRFS-SUBVOLUME-MANAGEMENT.md](../BTRFS-SUBVOLUME-MANAGEMENT.md) | Subvolume management | Admins |

### Implementation
Implementation details and reviews.

| Document | Description | Audience |
|----------|-------------|----------|
| [IMPLEMENTATION-COMPLETE.md](../IMPLEMENTATION-COMPLETE.md) | Implementation details | Developers |
| [CODE-REVIEW-REPORT.md](../CODE-REVIEW-REPORT.md) | Code review findings | Developers |
| [COUPLING-FIXES.md](../COUPLING-FIXES.md) | Architecture improvements | Developers |
| [PLUGTREE_PATTERN.md](PLUGTREE_PATTERN.md) | PlugTree pattern | Developers |

### Security
Security features and fixes.

| Document | Description | Audience |
|----------|-------------|----------|
| [SECURITY-FIXES.md](../SECURITY-FIXES.md) | Security improvements | Security engineers |

### Status & Maintenance
Project status and maintenance docs.

| Document | Description | Audience |
|----------|-------------|----------|
| [STATUS.md](../STATUS.md) | Project status | Maintainers |
| [DOCUMENTATION-INDEX.md](../DOCUMENTATION-INDEX.md) | Documentation index | Everyone |
| [DOCUMENTATION-CONSOLIDATION-PLAN.md](../DOCUMENTATION-CONSOLIDATION-PLAN.md) | Consolidation plan | Maintainers |

## 🎯 By Task

### I want to...

**Install op-dbus:**
→ [INSTALL-COMPLETE.md](../INSTALL-COMPLETE.md)

**Understand the architecture:**
→ [ARCHITECTURE-ANALYSIS.md](../ARCHITECTURE-ANALYSIS.md) + [MCP-ARCHITECTURE-ANALYSIS.md](../MCP-ARCHITECTURE-ANALYSIS.md)

**Integrate with MCP:**
→ [MCP-INTEGRATION.md](../MCP-INTEGRATION.md) + [MCP-COMPLETE-GUIDE.md](MCP-COMPLETE-GUIDE.md)

**Deploy to production:**
→ [ENTERPRISE-DEPLOYMENT.md](../ENTERPRISE-DEPLOYMENT.md)

**Set up containers:**
→ [CONTAINER-SETUP.md](../CONTAINER-SETUP.md)

**Configure networking:**
→ [DUAL-BRIDGE-ARCHITECTURE.md](../DUAL-BRIDGE-ARCHITECTURE.md)

**Understand caching:**
→ [INTROSPECTION-JSON-CACHE.md](INTROSPECTION-JSON-CACHE.md) (D-Bus introspection cache)
→ [BTRFS-CACHING-STRATEGY.md](../BTRFS-CACHING-STRATEGY.md) (BTRFS filesystem cache)

**Contribute code:**
→ [CODE-REVIEW-REPORT.md](../CODE-REVIEW-REPORT.md) + [COUPLING-FIXES.md](../COUPLING-FIXES.md)

## 📊 Documentation Statistics

- **Total Files:** 47 markdown files
- **Current & Accurate:** 41 files (87%)
- **Needs Update:** 5 files (11%)
- **Future Features:** 1 file (2%)

## ✅ Documentation Health

- ✅ Most documentation matches code implementation
- ✅ Architecture docs are current
- ✅ Deployment guides are accurate
- ⚠️ Some outdated path references (being fixed)
- ⚠️ Need reorganization (planned)

## 📝 Maintenance

This index is maintained by the project maintainers. To add or update documentation:

1. Create or update the relevant markdown file
2. Add entry to this index
3. Update cross-references
4. Verify links work

**Last Updated:** 2025-11-17
