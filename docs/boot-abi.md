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
       -> observe_platform + validate_platform
```

## Compatibility

Boot is refused when:

- magic mismatch
- unsupported `version`
- declared `size` inconsistent with the buffer
- configuration digest mismatch
- invalid descriptor bounds or ACPI RSDP signature

See `crates/hv-boot-abi/src/lib.rs` for authoritative constants and layout tests.
