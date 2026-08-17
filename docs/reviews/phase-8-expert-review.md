# Phase 8 expert review

Multi-domain review of hypervisor transfer, UEFI chain-load, and PCI firmware enumeration (`cursor/phase-8-hypervisor-transfer-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Transfer ABI | `hv-boot-abi::transfer`, `requirements_snapshot`, canonical layout enforcement |
| Loader / hypervisor host path | `build_hypervisor_transfer`, `boot_from_transfer`, `boot_from_transfer_snapshot` |
| UEFI boot chain | Loader publish + chain-load, hypervisor verify entry |
| Firmware observation | PCI config-space enumeration in loader collect path |
| Build / CI | `build-hypervisor-efi`, `build-boot-chain`, fuzz target, UEFI clippy |

## Findings and disposition

### MUST FIX (applied)

1. **Hypervisor transfer ABI** — Versioned `HVTFR` blob with boot-info and observation sections, published through `HV_TRANSFER_TABLE_GUID`.
2. **UEFI chain-load** — Loader publishes transfer into runtime configuration-table memory and chain-loads `hv-hypervisor.efi`.
3. **Hypervisor UEFI entry** — `hv-hypervisor-efi-bin` embeds digest + requirements snapshot and verifies the transfer before returning `EFI_SUCCESS`.
4. **CI formatting** — `cargo fmt --check` enforced on review branch.

### SHOULD FIX (applied)

5. **PCI enumeration** — Loader firmware collect scans PCI config space and forwards discovered BDFs in the transfer payload.
6. **Host snapshot boot path** — `boot_from_transfer_snapshot()` restores `PlatformRequirements` from embedded snapshot metadata for host integration tests.
7. **Build tooling** — `cargo xtask build-hypervisor-efi` and `cargo xtask build-boot-chain` produce OVMF-ready images under `build/`.
8. **Transfer canonical layout** — `HypervisorTransferView::parse` rejects non-contiguous section layouts (boot info must follow header; observation must follow boot info; total size must match section span).
9. **UEFI loader clippy** — Fix precedence and unused-assignment warnings in firmware collect path (`-D warnings` clean on `x86_64-unknown-uefi` loader build).
10. **Documentation** — PCI enumeration limits documented in `docs/ovmf-boot.md`; `transfer_parse` listed in `docs/fuzzing.md`.

### Documented (deferred)

11. **Full platform validation on UEFI** — Hypervisor firmware entry verifies transfer structure and digests only; VMX/EPT/VT-d enablement and full observed-platform validation remain host-tested until Phase 9+.
12. **OVMF CI runtime boot** — CI builds boot-chain images; QEMU/OVMF runtime smoke boot remains manual (`docs/ovmf-boot.md`).
13. **Production PCI discovery** — No ECAM/`PciRootBridgeIo`, no bridge recursion, bus walk stops at first empty bus; sufficient for reference QEMU topology only.
14. **Transfer slice hardening** — Hypervisor UEFI entry trusts `header.total_size` from runtime memory when forming the slice; allocation-size binding is Phase 9+ hardening.
15. **Requirements snapshot metadata** — `partition_id` / device `kind` strings omitted from snapshot; BDF + vm_id validation only today.
16. **UEFI crates in workspace CI** — Standalone `*-efi-bin` manifests build via xtask/CI jobs but are outside root `cargo clippy --workspace`.

## Delivered

| Component | Role |
|-----------|------|
| `hv-boot-abi::transfer` | Transfer blob encode/decode, canonical layout validation, configuration-table GUID |
| `hv-boot-abi::requirements_snapshot` | Fixed-size requirements metadata for UEFI hypervisor images |
| `hv-loader-efi-bin` | Publish transfer, chain-load hypervisor, PCI enumeration |
| `hv-hypervisor-efi` | Portable transfer verification entry (host-tested) |
| `hv-hypervisor-efi-bin` | UEFI hypervisor application (`hv-hypervisor.efi`) |
| `cargo xtask build-boot-chain` | Build loader + hypervisor images for OVMF |

## Verification

- `cargo test --workspace` — pass
- `cargo clippy --all-targets --all-features -- -D warnings` — pass
- `cargo xtask coverage` — pass (≥ 95% line coverage)
- `cargo xtask build-boot-chain` — produces `build/boot-chain/hv-loader.efi` and `hv-hypervisor.efi`
- `cargo clippy --release --target x86_64-unknown-uefi --manifest-path crates/hv-loader-efi-bin/Cargo.toml -- -D warnings` — pass

## Review status

All MUST and SHOULD items above are applied. Deferred items are documented with explicit phase ownership. PR **#7** is ready for human review.
