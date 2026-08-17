# Phase 8 expert review

Multi-domain review of hypervisor transfer, UEFI chain-load, and PCI firmware enumeration (`cursor/phase-8-hypervisor-transfer-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Transfer ABI | `hv-boot-abi::transfer`, requirements snapshot embedding |
| Loader / hypervisor host path | `build_hypervisor_transfer`, `boot_from_transfer`, `boot_from_transfer_snapshot` |
| UEFI boot chain | Loader publish + chain-load, hypervisor verify entry |
| Firmware observation | PCI config-space enumeration in loader collect path |
| Build / CI | `build-hypervisor-efi`, `build-boot-chain`, fuzz target |

## Findings and disposition

### MUST FIX (applied)

1. **Hypervisor transfer ABI** — Versioned `HVTFR` blob with boot-info and observation sections, published through `HV_TRANSFER_TABLE_GUID`.
2. **UEFI chain-load** — Loader publishes transfer into runtime configuration-table memory and chain-loads `hv-hypervisor.efi`.
3. **Hypervisor UEFI entry** — `hv-hypervisor-efi-bin` embeds digest + requirements snapshot and verifies the transfer before returning `EFI_SUCCESS`.

### SHOULD FIX (applied)

4. **PCI enumeration** — Loader firmware collect scans PCI config space and forwards discovered BDFs in the transfer payload.
5. **Host snapshot boot path** — `boot_from_transfer_snapshot()` restores `PlatformRequirements` from embedded snapshot metadata for host integration tests.
6. **Build tooling** — `cargo xtask build-hypervisor-efi` and `cargo xtask build-boot-chain` produce OVMF-ready images under `build/`.

### Documented (deferred)

7. **Full platform validation on UEFI** — Hypervisor firmware entry verifies transfer structure and digests only; VMX/EPT/VT-d enablement and full observed-platform validation remain host-tested until Phase 9+.
8. **OVMF CI runtime boot** — CI builds boot-chain images; QEMU/OVMF runtime smoke boot remains manual (`docs/ovmf-boot.md`).

## Delivered

| Component | Role |
|-----------|------|
| `hv-boot-abi::transfer` | Transfer blob encode/decode and configuration-table GUID |
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

## Review status

All MUST and SHOULD items above are applied. Deferred items are documented with explicit phase ownership. PR **#7** is ready for human review.
