# Phase 51 expert review — PCI BAR MMIO from platform description

## Scope

Replace synthetic vm-id-based e1000 MMIO guest physical planning with explicit BAR addresses from platform configuration.

## Changes

| Area | Change |
|------|--------|
| `configs/qemu.yaml` | `mmio_guest_phys` on each `nic_e1000` device |
| `hv-config-model` | `parse_guest_phys`, schema + semantic page-align validation |
| `hv-platform-model` | `PlannedPciDevice.mmio_guest_phys/mmio_size_bytes`, MMIO lookup helpers |
| `hv-datapath` | `plan_e1000_mmio_guest_phys(layout, vm_id)`, relay page after out BAR |
| `hv-boot-abi` | `LayoutPciSnapshot` encodes MMIO BAR metadata |
| `tools/hv-config` | Embedded config codegen emits MMIO fields |
| `guests/guest-common` | Reference OUT MMIO aligned with platform (`0xFEB20000`) |
| `xtask` | Relay-measurement coverage pass uses `RUSTFLAGS=--cfg=coverage` |

## Usage

MMIO BAR bases are declared per device in YAML and flow config → intent → platform IR → layout snapshot → Gate D/EPT/guest boot info.

```yaml
devices:
  - kind: nic_e1000
    bdf: "0000:00:03.0"
    role: datapath_in
    mmio_guest_phys: "0xFEB00000"
```

## Still deferred

- Outer QEMU BAR discovery at runtime (config remains source of truth)
- Descriptor rings, DMA, live tap frame I/O
- Concurrent in/mid/out partition scheduling

## Verification

- `cargo test --workspace`
- `cargo clippy --all-targets --all-features`
- `cargo xtask build-boot-chain --config configs/ovmf-smoke.yaml`
