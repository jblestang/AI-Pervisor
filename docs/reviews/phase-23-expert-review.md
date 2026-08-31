# Phase 23 expert review

Multi-domain review of Gate D datapath guest sources: real `guests/` source-tree ELFs, `datapath-guest-sources` feature chain, and host live wall-clock benchmark harness (`cursor/phase-23-guest-sources-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Guest source trees | `guests/guest-{in,mid,out}`, `guest-common` IPC/e1000/boot-info |
| Source ELF embedding | `hv-guest-boot/build.rs`, `GuestElfKind::Source`, `GUEST_*_SOURCE_ELF` |
| Gate D guest-sources | `GateDDatapathGuestSourcesResult`, `boot_*_gate_d_datapath_guest_sources*()` |
| UEFI + xtask | `datapath-guest-sources` feature chain, serial markers, coverage pass |
| Host live benchmark | `cargo xtask build-guests`, `cargo xtask datapath-live-benchmark` |

## Phase 22 deferrals closed

| Phase 22 item | Phase 23 disposition |
|---------------|---------------------|
| Real `guests/` source trees | **Partially closed** — freestanding in/mid/out guests + Gate D source ELF install |
| Live 200 Mbit/s under VMX | **Unchanged** — host wall-clock harness only; firmware keeps deterministic mock timing |

## Issues found and fixed

| Issue | Fix |
|-------|-----|
| UEFI not wired for `datapath-guest-sources` | Added feature chain on `hv-hypervisor-efi` / `hv-hypervisor-efi-bin`, `boot_hypervisor_from_transfer_datapath_guest_sources()`, serial marker logging, EFI integration test |
| Guest reference IPC layout wrong (`0x4000_0000` vs planner `0x1_C000_0000`) | Corrected `guest-common` reference constants; added `reference_guest_ipc_layout_matches_planner` test in `hv-datapath` |
| Redundant fragile embedded-ELF test | Boot/EFI tests skip gracefully when `build-guests` has not been run |
| Missing xtask constant / coverage pass | Added `HYPERVISOR_EFI_DATAPATH_GUEST_SOURCES_FEATURE`; coverage and live boot-chain include guest-sources |
| Dead `resolve_layout()` in guest-common | Removed unused wrapper; kept `resolve_layout_for_role()` |

## Deferred to Phase 24+

| Item | Reason |
|------|--------|
| Boot-info blob install + `GUEST_RDI` at VMX entry | Requires hypervisor guest-launch plumbing beyond source ELF install |
| Live VMX execution of source-tree guests | Gate D still validates via host-side `run_guest_datapath_runtime()` |
| In-VM 200 Mbit/s throughput measurement | Host wall-clock harness remains the proof path |

## Feature matrix

| Feature | Crate | Default | UEFI chain | Effect |
|---------|-------|---------|------------|--------|
| `datapath-guest-sources` | `hv-hypervisor-boot` | off | off | Gate D runtime using built `guests/` ELFs |
| `datapath-guest-sources` | `hv-hypervisor-efi` | off | opt-in | Guest-sources boot entry + source ELF marker |
| `datapath-guest-sources` | `hv-hypervisor-efi-bin` | off | opt-in | Builds on `datapath-runtime` |

## Serial markers

- `GATE_D_GUEST_SOURCE_ELF_MARKER` — source-tree guest ELFs installed for all partitions
- Inherited runtime markers from Phase 22 (`GATE_D_DATAPATH_RUNTIME_MARKER`, `GATE_D_GUEST_DATAPATH_FRAME_MARKER`, etc.)

## Verification

- `cargo xtask build-guests` — builds and stages `guests/guest-*/build/*.elf`
- `cargo test -p hv-datapath` — reference IPC layout alignment test
- `cargo test -p hv-hypervisor-boot --features datapath-guest-sources` — pass (after `build-guests`)
- `cargo test -p hv-hypervisor-efi --features datapath-guest-sources` — pass (after `build-guests`)
- `cargo xtask datapath-live-benchmark` — build guests + host wall-clock benchmark

## Review status

Phase 23 closes the real guest source-tree scaffolding gap from Phase 22: freestanding partition guests, xtask build/install path, Gate D orchestration with source ELF embedding, and UEFI serial-marker parity with prior datapath phases. Live VMX guest execution, boot-info handoff at guest entry, and in-VM throughput measurement remain deferred to Phase 24+.
