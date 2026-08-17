# Phase 7 expert review

Multi-domain review of UEFI `.efi` loader build and OVMF integration (`cursor/phase-7-uefi-binary-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| UEFI / firmware entry | `hv-loader-efi-bin`, memory map, RSDP, CPUID collection |
| Boot ABI / loader handoff | `build_loader_handoff` with `PhysicalMemory`, digest embedding |
| Dependency / no_std | `hv-observation-types`, loader crate `std` feature split |
| Build / CI | `cargo xtask build-efi`, OVMF docs |

## Findings and disposition

### MUST FIX (applied)

1. **`no_std` dependency chain** — Extracted `hv-observation-types` so the UEFI loader no longer pulls `hv-config-model` through `hv-platform-model`.
2. **Handoff physical memory parameter** — `build_loader_handoff()` now accepts `&impl PhysicalMemory`, enabling identity-mapped firmware reads in the UEFI binary instead of a host `FirmwareMemoryImage` only.

### SHOULD FIX (applied)

3. **`ToString` in no_std loader errors** — Import `alloc::string::ToString` in loader/EFI error mapping so UEFI builds format errors without `std`.
4. **Config digest embedding** — `hv-loader-efi-bin/build.rs` reads `build/config.sha256` (override via `HV_CONFIG_DIGEST_PATH`) and embeds a `CONFIG_DIGEST` constant.

### Documented (deferred)

5. **PCI enumeration in firmware** — UEFI entry passes an empty PCI list; expected devices are still validated on the host integration path until firmware PCI discovery lands.
6. **Hypervisor transfer** — Successful handoff returns `EFI_SUCCESS`; transferring the boot-info blob to the hypervisor image is Phase 8+.
7. **OVMF CI boot** — CI builds the `.efi` image; full QEMU/OVMF runtime boot remains manual (documented in `docs/ovmf-boot.md`).

## Delivered

| Component | Role |
|-----------|------|
| `hv-observation-types` | `CpuidSnapshot`, `ObservationInputs` (`no_std` + `alloc`) |
| `hv-loader-efi-bin` | UEFI app: collect firmware inputs, run `uefi_loader_entry`, build `hv-loader.efi` |
| `cargo xtask build-efi` | Generate config digest, build UEFI target, copy to `build/hv-loader.efi` |
| `docs/ovmf-boot.md` | OVMF/QEMU smoke boot instructions |

## Verification

- `cargo test --workspace` — pass
- `cargo clippy --all-targets --all-features -- -D warnings` — pass
- `cargo xtask coverage` — pass (95.20% line coverage; manual threshold enforcement in xtask because `cargo-llvm-cov` 0.6.21 ignores `--fail-under-lines`)
- `cargo xtask build-efi` — produces `build/hv-loader.efi`
