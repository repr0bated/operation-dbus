# Repository Guidelines

## Project Structure & Module Organization
- `src/` — Rust 2021 code
  - `src/main.rs` — CLI entry (`op-dbus`)
  - `src/state/` — plugin framework (`manager.rs`, `plugin.rs`, `plugins/` with `net.rs`, `systemd.rs`)
  - `src/native/` — direct protocol clients (`ovsdb_jsonrpc.rs`, `rtnetlink_helpers.rs`)
  - `src/blockchain/` — footprint + streaming audit
- Scripts: `build.sh`, `install.sh`, `test-safe.sh`, `test-introspection.sh`
- Config examples: `example-state.json`

## Build, Test, and Development Commands
- Build release: `cargo build --release` (or `./build.sh`)
- Run binary: `target/release/op-dbus ...`
- Unit tests: `cargo test -q`
- Safe system tests (read-only): `sudo ./test-safe.sh`
- Introspection demo: `./test-introspection.sh`
- Logging: `RUST_LOG=op_dbus=debug op-dbus query`

## Coding Style & Naming Conventions
- Rust: rustfmt defaults (4-space indent). Run `cargo fmt` before PRs; lint with `cargo clippy -- -D warnings` when feasible.
- Names: modules/files `snake_case`; types/enums `CamelCase`; functions/vars `snake_case`; constants `SCREAMING_SNAKE_CASE`.
- JSON state schema: keys in `snake_case`; enum string values may be `kebab-case` (e.g., `"type": "ovs-bridge"`).
- Shell: Bash, `set -e` (prefer `-euo pipefail` for new scripts); filenames `kebab-case`.

## Testing Guidelines
- Prefer focused unit tests alongside code (`#[cfg(test)]` in `src/**`). Integration tests may go in `tests/`.
- Required: all tests pass via `cargo test`. No minimum coverage enforced; add tests for new plugins, diff logic, and error paths.
- For system-affecting changes, show `op-dbus diff` output before `apply` in the PR description.

## Commit & Pull Request Guidelines
- Commits: concise, imperative subject (e.g., "add net diff for ports"). Group related changes.
- PRs must include:
  - What/why summary and linked issue (if any)
  - How to test (commands, expected output)
  - Risk notes (systemd/OVS impact) and rollback plan
  - For CLI changes: sample `op-dbus query/diff/apply` output

## Security & Configuration Tips
- Many actions require root and system sockets: `/var/run/dbus/system_bus_socket`, `/var/run/openvswitch/db.sock`.
- Always validate with `op-dbus diff /etc/op-dbus/state.json` before `apply`. Avoid enabling the service until manual tests succeed.
- Use a throwaway host/VM for new network/systemd behaviors.
