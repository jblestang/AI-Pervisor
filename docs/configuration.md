# Configuration

## Source of truth

[`configs/qemu.yaml`](../configs/qemu.yaml) is the single source of truth for:

- partitions
- CPU affinity and SMT policy
- RAM sizes
- PCI ownership
- IPC topology
- platform requirements
- QEMU launch parameters
- boot image metadata

The same data must never be duplicated manually in Rust, QEMU scripts, or benchmark scripts.

## Deterministic IDs

After normalization:

1. Partitions are sorted lexicographically by `id`.
2. `VmId` values are assigned densely from `0..n-1` in sorted order.
3. IPC channels are sorted lexicographically by `id`.
4. `IpcChannelId` values are assigned densely from `0..m-1` in sorted order.

Renaming a partition changes IDs. Reordering YAML keys does not change IDs or hash.

## Configuration hash

The official digest is:

```text
SHA-256(JSON(NormalizedConfig))
```

Properties:

- keys are sorted by serde_json
- no insignificant whitespace
- independent of YAML formatting

The digest is written to `build/config/config.sha256` by `cargo xtask config generate`.

Phase 4 additionally emits:

- `static-platform-layout.json` — resolved host physical layout (`StaticPlatformIR`)
- `platform-layout.txt` — human-readable layout summary

## Validation

Validation is fail-closed:

- unknown YAML fields are rejected
- unsupported schema versions are rejected
- duplicate partition, IPC, PCI, or CPU assignments are rejected
- IPC graphs must be acyclic and unidirectional

## Commands

```bash
cargo xtask config validate configs/qemu.yaml
cargo xtask config generate configs/qemu.yaml
```
