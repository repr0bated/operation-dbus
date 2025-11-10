# Codex Initialization - Quick Start

This repository contains a comprehensive initialization statement for Codex to work on the Proxmox-to-NixOS conversion project.

## 📄 Main Document

**Primary File:** [`CODEX-INIT-STATEMENT.md`](./CODEX-INIT-STATEMENT.md)

This 1000+ line document provides Codex with:
- Complete project context and goals
- Technical architecture overview
- 7-day work plan with daily milestones
- Comprehensive pitfall analysis framework
- Code structure requirements
- Testing strategy
- Success criteria

## 🎯 Project Summary

**Goal:** Create a Rust program that converts a Proxmox VE installation to use Nix/PackageKit/zbus.

**Duration:** 1 week (7 days)

**Environment:** VM accessible via `pct enter 1000` with NixOS installed

**Key Requirements:**
- 100% D-Bus/PackageKit based (no direct apt/dpkg calls)
- Identify ALL possible pitfalls
- Write Rust bridge code for gaps
- Test thoroughly with rollback capability

## 🚀 For Codex: Where to Start

1. **Read:** `CODEX-INIT-STATEMENT.md` in full
2. **Access:** VM via `pct enter 1000`
3. **Navigate:** `cd /root/operation-dbus`
4. **Begin:** Follow Day 1 tasks in the initialization statement

## 📊 Expected Deliverables

By end of 7 days:
- ✅ Rust converter tool (`src/bin/proxmox-to-nix.rs`)
- ✅ Pitfall documentation (100+ cases)
- ✅ Gap analysis report (JSON)
- ✅ Bridge code for all gaps
- ✅ Test report
- ✅ User guide
- ✅ Example NixOS configurations

## 🎓 Key Context Files to Reference

- `src/state/plugins/packagekit.rs` - Existing PackageKit integration
- `nix/module.nix` - NixOS module structure
- `nix/PROXMOX.md` - Proxmox deployment guide
- `ARCHITECTURE-ANALYSIS.md` - System architecture

## 🔧 Critical Constraints

1. **D-Bus Only:** All package operations via PackageKit/zbus
2. **Safe:** Always create checkpoints before changes
3. **Tested:** Test everything in VM before proceeding
4. **Documented:** Write down every finding

## 📞 Support

See `CODEX-INIT-STATEMENT.md` sections:
- 🔍 Pitfall Analysis Framework
- 💻 Rust Code Requirements
- 🧩 Gap Bridging Strategy
- 📋 Deliverables
- 🎓 Learning Resources

---

**Ready to begin!** 🚀

Clone this repo and point Codex to `CODEX-INIT-STATEMENT.md`.
