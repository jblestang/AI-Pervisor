# Boot ABI

Version: `BOOT_ABI_VERSION = 1`

## Purpose

Defines the stable handoff from `hv-loader.efi` to the hypervisor.

## Layout rules

- `#[repr(C)]` structures only
- no heap types, references, or Rust metadata
- self-describing tables via `(offset, size, kind)` descriptors

## Core types

- `BootInfoHeader`
- `BootInfoDescriptor`

## Compatibility

Boot is refused when:

- magic mismatch
- unsupported `version`
- `size` smaller than header
- configuration digest mismatch (future loader policy)

See `crates/hv-boot-abi/src/lib.rs` for authoritative constants and layout tests.
