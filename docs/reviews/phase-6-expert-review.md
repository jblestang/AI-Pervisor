# Phase 6 expert review

Multi-domain review of Gate B ACPI discovery and portable UEFI loader entry (`cursor/phase-6-uefi-loader-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| ACPI / firmware walk | `hv-acpi-walk`, RSDP → XSDT/RSDT, checksums, bounds |
| Boot ABI / loader handoff | `hv-loader`, `hv-loader-efi`, boot info + observation bundle |
| Rust safety / no-panic | allocation limits, error propagation, fixture helpers |
| Test / coverage | host firmware fixtures, integration paths |

## Findings and disposition

### MUST FIX (applied)

1. **ACPI declared-length cap** — `read_table()` rejects declared table lengths above `ACPI_TABLE_MAX_LENGTH` (1 MiB) before allocation. Prevents hostile firmware from forcing unbounded host allocations during the walk.
2. **Collected ACPI output cap** — `append_nested_tables()` rejects walks that would exceed `ACPI_COLLECTED_MAX_BYTES` (16 MiB) or `ACPI_ROOT_MAX_ENTRIES` (256 pointers). Keeps loader handoff bounded and fail-closed.

### SHOULD FIX (applied)

3. **Firmware fixture writes** — `write_at()` in `hv-loader` test fixtures now panics on out-of-bounds writes instead of silently skipping, so broken fixture layouts fail loudly in tests.
4. **Physical memory copy** — `FirmwareMemoryImage::read_physical()` uses `copy_from_slice` on the bounded slice instead of per-byte iteration.

### Documented (deferred)

5. **`.efi` binary packaging** — `hv-loader-efi` is host-tested only; `x86_64-unknown-uefi` build and OVMF boot remain Phase 7+.
6. **Runtime UEFI services in firmware entry** — memory map, CPUID, and PCI enumeration are still supplied by the caller; the portable entry wraps `build_loader_handoff()` only.
7. **Duplicate RSDP parse** — `build_loader_handoff()` validates via `validate_rsdp_section()` then parses again for the walk. Acceptable for now; can fold into one parse later without behavior change.

## Delivered

| Component | Role |
|-----------|------|
| `hv-acpi-walk` | `PhysicalMemory` trait, `FirmwareMemoryImage`, bounded `collect_acpi_tables()` via XSDT (preferred) or RSDT |
| `hv-loader` | Handoff derives `acpi_tables` from RSDP + firmware memory; QEMU reference fixture |
| `hv-loader-efi` | `uefi_loader_entry()` portable entry wrapping `build_loader_handoff()` |
| `hv-boot-abi` | `finalize_acpi_table_checksum()`, `AcpiRsdp::encode_reference_v2_with_xsdt()` |

## Verification

- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo xtask coverage` ≥ 95%
