# Phase 5 expert review

Multi-domain review of Gate B boot path (`cursor/phase-5-boot-path-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Boot ABI / UEFI handoff | `hv-boot-abi`, `hv-loader` |
| ACPI / firmware | DMAR scan, RSDP validation |
| x86 CPUID / platform | `CpuidSnapshot`, `observe_platform` |
| Rust safety / no-panic | error propagation, bounds checks |

## Findings and disposition

### MUST FIX (applied)

1. **DMAR flags offset** — `DMAR_FLAGS_OFFSET` corrected from `0x28` to `0x25` (Intel VT-d layout). Fixtures updated via `encode_reference_dmar_with_intr_remap()`.
2. **DMAR length bounds** — `scan_acpi_capabilities()` now validates declared table length before reading flags; rejects truncated or over-long tables.
3. **Boot info declared-size enforcement** — `BootInfoView::validate_layout()` confines descriptor table and section reads to `header.size`; `descriptor()` / `section()` use bounded slices.

### SHOULD FIX (applied)

4. **RSDP checksum validation** — `AcpiRsdp::parse()` validates ACPI 1.0 and 2.0+ checksums; `validate_rsdp_section()` delegates to it. Reference encoder: `AcpiRsdp::encode_reference_v2()`.
5. **Memory map cross-check** — `boot_check()` rejects mismatches between boot-info memory-map section and `ObservationInputs::memory_map`.
6. **Shared UEFI descriptor parse** — `UefiMemoryDescriptor::parse()` in `hv-boot-abi`; observation uses it instead of duplicated offsets.
7. **40-byte descriptor stride** — observation accepts UEFI minimum descriptor size (40 bytes), not only 48-byte OVMF stride.
8. **Loader input validation order** — `memory_descriptor_size` validated before boot-info construction.

### Documented (deferred)

9. **Flattened ACPI table contract** — Phase 5 accepts a loader-flattened ACPI byte stream. Walking RSDP → XSDT/RSDT is planned for the UEFI loader binary (Phase 6+).

## Verification

- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo xtask coverage` ≥ 95%
