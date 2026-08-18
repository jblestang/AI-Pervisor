# Phase 17 expert review

Multi-domain review of Gate D datapath foundation: layout snapshot IPC/device identifiers, datapath planning, full guest boot info construction, Gate D `datapath-foundation` orchestration, and KVM smoke harness extensions (`cursor/phase-17-datapath-foundation-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Layout snapshot ABI | `LayoutGuestRegionSnapshot`, `LayoutIpcRegionSnapshot`, `device_kind` on PCI entries |
| Snapshot round-trip | `vm_id`, `channel_id`, producer/consumer VM ids, device kind preservation |
| Datapath planning | `hv-datapath::plan_datapath_for_partition`, e1000 MMIO guest phys |
| Guest boot info | Full IPC + device tables, `GuestBootInfoView` parser |
| Gate D orchestration | `GateDDatapathFoundationResult`, `boot_*_gate_d_datapath_foundation*()` |
| UEFI datapath path | `boot_hypervisor_from_transfer_datapath_foundation()`, serial markers |
| Build / CI | Multi-pass coverage (`datapath-foundation` feature passes) |

## Phase 16 deferrals closed

| Phase 16 item | Phase 17 disposition |
|---------------|---------------------|
| Full IPC/NIC datapath in boot info | **Partially closed** — IPC + e1000 MMIO descriptors for all partitions |
| Layout snapshot missing channel/vm/kind IDs | **Closed** — compact identifiers in layout snapshot |
| e1000 datapath (Gate D) | **Partially closed** — MMIO planning + device descriptors; live TX/RX deferred |
| Live IPC forwarding | **Deferred** |
| Malicious guest tests | **Deferred** |
| 200 Mbit/s performance benchmark | **Deferred** |
| DMAR MMIO | **Unchanged** |
| Multi-partition VMLAUNCH | **Deferred** — still launches smoke guest on `in` only |

## Feature matrix

| Feature | Crate | Default | UEFI chain | Effect |
|---------|-------|---------|------------|--------|
| `datapath-foundation` | `hv-hypervisor-boot` | off | off | Gate D orchestration atop `vmx-launch` |
| `datapath-foundation` | `hv-hypervisor-efi` | off | opt-in | Datapath foundation boot entry + markers |
| `datapath-foundation` | `hv-hypervisor-efi-bin` | off | opt-in | Builds on `vmx-launch` |
| `datapath-foundation` | `hv-datapath` | n/a | n/a | IPC role + e1000 MMIO planning |

## Verification

- `cargo test --workspace` — pass
- `cargo test -p hv-hypervisor-boot --features datapath-foundation` — pass
- `cargo test -p hv-hypervisor-efi --features datapath-foundation` — pass
- `cargo test -p hv-datapath` — pass
- `cargo test -p hv-guest-boot` — pass
- `cargo clippy --all-targets --all-features -- -D warnings` — pass
- `cargo xtask coverage` — multi-pass includes `datapath-foundation` (line coverage ~92%; below 95% gate, consistent with Phase 15–16)
- `cargo xtask build-boot-chain-live` — builds with `real-hw-execution,vmx-launch,datapath-foundation`
- `cargo xtask live-qemu-smoke` — pass or skip (exit 0) when KVM/VMX unavailable

## Review status

Phase 17 closes the layout snapshot identifier gap from Phase 11 and extends guest boot info from smoke-only RAM tables to full IPC + e1000 MMIO descriptors for all three reference partitions. Live datapath execution (e1000 packet path, IPC queue runtime, malicious tests, performance) remains deferred to later Gate D phases.
