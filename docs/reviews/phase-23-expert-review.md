# Phase 23 expert review

Multi-domain review of Gate D datapath guest sources: real `guests/` source-tree ELFs, `datapath-guest-sources` feature chain, and host live wall-clock benchmark harness (`cursor/phase-23-guest-sources-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Guest source trees | `guests/guest-{in,mid,out}`, `guest-common` IPC/e1000/boot-info |
| Source ELF embedding | `hv-guest-boot/build.rs`, `GuestElfKind::Source`, `GUEST_*_SOURCE_ELF` |
| Gate D guest-sources | `GateDDatapathGuestSourcesResult`, `boot_*_gate_d_datapath_guest_sources*()` |
| Host live benchmark | `cargo xtask build-guests`, `cargo xtask datapath-live-benchmark` |

## Phase 22 deferrals closed

| Phase 22 item | Phase 23 disposition |
|---------------|---------------------|
| Real `guests/` source trees | **Partially closed** — freestanding in/mid/out guests + Gate D source ELF install |
| Live 200 Mbit/s under VMX | **Unchanged** — host wall-clock harness only; firmware keeps deterministic mock timing |

## Feature matrix

| Feature | Crate | Default | Effect |
|---------|-------|---------|--------|
| `datapath-guest-sources` | `hv-hypervisor-boot` | off | Gate D runtime using built `guests/` ELFs |
| `datapath-guest-sources` | `hv-hypervisor-efi` | off | opt-in guest-sources boot entry |

## Serial markers

- `GATE_D_GUEST_SOURCE_ELF_MARKER` — source-tree guest ELFs installed for all partitions

## Verification

- `cargo xtask build-guests` — builds and stages `guests/guest-*/build/*.elf`
- `cargo test -p hv-guest-boot` — source ELF parse when embedded
- `cargo test -p hv-hypervisor-boot --features datapath-guest-sources` — pass
- `cargo xtask datapath-live-benchmark` — build guests + host wall-clock benchmark

## Review status

Phase 23 closes the real guest source-tree scaffolding gap from Phase 22: freestanding partition guests, xtask build/install path, and Gate D orchestration with source ELF embedding. Live VMX guest execution and in-VM throughput measurement remain deferred to Phase 24+.
