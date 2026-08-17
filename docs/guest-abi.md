# Guest ABI

Version: `GUEST_ABI_VERSION = 1`

## Purpose

Defines how a guest discovers its identity and resources at boot.

Guests must not depend on hardcoded constants such as `const IPC_ADDR: usize = ...`.

## Layout rules

- `#[repr(C)]` structures only
- resources described by tables in guest physical memory
- IPC roles encoded as `Producer` or `Consumer`

## Core types

- `GuestBootInfoHeader`
- `GuestMemoryRegion`
- `GuestIpcRegion`
- `GuestDeviceRegion`

## Compatibility

Guests must refuse to continue when the header magic or version is unsupported.

See `crates/hv-guest-abi/src/lib.rs` for authoritative constants and layout tests.
