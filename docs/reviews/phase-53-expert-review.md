# Phase 53 expert review — outer QEMU e1000 BAR0 discovery

## Scope

Read outer e1000 BAR0 bases from PCI config space at runtime and validate them against the Phase 52 host-network BDF contract.

## Changes

| Area | Change |
|------|--------|
| `hv-x86-cpu` | `pci_config.rs`: CF8/CFC read, BAR0 decode, e1000 vendor/device check |
| `hv-datapath` | `DiscoveredOuterHostBars`, `validate_discovered_outer_host_bars` |
| `hv-hypervisor-boot` | Gate D relay path discovers/validates when `host_network.enabled` under REAL_HW |
| `hv-boot-abi` | `REAL_HW_OUTER_HOST_BAR0_DISCOVERED_MARKER` serial marker constant |
| `xtask` | Document deferred QEMU BAR pinning until `fixed-bars`/`pci-bars` is available |

## Usage

Under REAL_HW with host networking enabled, Gate D reads PCI config at the contract IN/OUT BDFs, requires Intel e1000 (8086:100e), and validates non-zero page-aligned distinct non-overlapping MMIO BAR0 windows.

Nested guest MMIO GPAs remain config-driven (Phase 51–52). Outer BAR discovery proves hardware presence at contract BDFs without requiring equality to nested GPA until QEMU BAR pinning lands.

## Still deferred

- QEMU `fixed-bars` / `pci-bars` pinning so outer BAR addresses match platform MMIO
- Descriptor rings, DMA, live tap frame I/O
- Concurrent in/mid/out partition scheduling

## Verification

- `cargo test --workspace`
- `cargo clippy --all-targets --all-features`
- `cargo xtask build-boot-chain --config configs/ovmf-smoke.yaml`
- Opt-in live: `cargo xtask live-qemu-smoke --require-executed --no-skip --build`
