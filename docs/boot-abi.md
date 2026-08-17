# Boot ABI

Version: `BOOT_ABI_VERSION = 1`

## Purpose

Defines the stable handoff from `hv-loader.efi` to the hypervisor.

Phase 5 adds parse-only validation in `hv-boot-abi` and host-side blob construction in `hv-loader`. The UEFI binary itself remains future work.

## Layout rules

- `#[repr(C)]` structures only
- no heap types, references, or Rust metadata cross the ABI boundary
- self-describing tables via `(offset, size, kind)` descriptors

## Core types

- `BootInfoHeader`
- `BootInfoDescriptor`
- `BootInfoView` (parse-only borrowed view over a boot info blob)
- `AcpiRsdp`, `AcpiTableHeader`, `UefiMemoryDescriptor`

## Boot path (Phase 5)

```text
LoaderHandoffInput (firmware snapshot)
  -> hv-loader::build_loader_handoff()
       -> boot info blob + ObservationInputs
  -> hv-hypervisor::boot_check()
       -> BootInfoView::parse + verify_config_digest
       -> RSDP checksum validation (AcpiRsdp::parse)
       -> memory-map section cross-check
       -> observe_platform + validate_platform
```

RSDP validation covers signature plus ACPI 1.0 / 2.0+ checksums. DMAR interrupt-remapping detection uses `DMAR_FLAGS_OFFSET = 0x25` and respects declared table length.

See [Phase 5 expert review](reviews/phase-5-expert-review.md) for review findings and fixes.

## Compatibility

Boot is refused when:

- magic mismatch
- unsupported `version`
- declared `size` inconsistent with the buffer
- configuration digest mismatch
- invalid descriptor bounds or ACPI RSDP signature

See `crates/hv-boot-abi/src/lib.rs` for authoritative constants and layout tests.
