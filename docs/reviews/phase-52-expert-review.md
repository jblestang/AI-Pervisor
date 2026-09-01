# Phase 52 expert review — host network PCI/MMIO coherence

## Scope

Bind outer QEMU `host_network` descriptions to partition PCI/MMIO layout so host NIC metadata stays coherent with nested e1000 BAR planning.

## Changes

| Area | Change |
|------|--------|
| `hv-config-model` | Semantic validation links `qemu.network.interfaces` to partition `nic_e1000` BDFs and datapath roles |
| `hv-platform-model` | `HostNetworkInterface.mmio_guest_phys` populated from PCI intent; `validate_layout_host_network_coherence` |
| `hv-datapath` | `plan_e1000_host_attach` invokes layout coherence validation |
| `hv-boot-abi` | `LayoutHostNetworkSnapshot` encodes MMIO BAR base |
| `tools/hv-config` | Embedded config codegen emits host network MMIO |
| `hv-hypervisor-boot` | Layout snapshot roundtrip preserves host network MMIO |

## Usage

Host network interfaces inherit MMIO BAR bases from the matching partition PCI device at plan time:

```yaml
qemu:
  network:
    interfaces:
      - partition: in
        bdf: "0000:00:03.0"
partitions:
  - id: in
    devices:
      - kind: nic_e1000
        bdf: "0000:00:03.0"
        mmio_guest_phys: "0xFEB00000"
```

## Still deferred

- Runtime outer QEMU BAR discovery from PCI config space
- Descriptor rings, DMA, live tap frame I/O
- Concurrent in/mid/out partition scheduling

## Verification

- `cargo test --workspace`
- `cargo clippy --all-targets --all-features`
- `cargo xtask build-boot-chain --config configs/ovmf-smoke.yaml`

## Review fixes

- Semantic validation requires `tap_ifname` when backend is `tap`; rejects duplicate partition entries
- `mmio_guest_phys_for_bdf` resolves `nic_e1000` devices only; coherence checks PCI kind
- Layout snapshot encode/decode fails closed on zero host network MMIO
- Snapshot restore validates host network coherence against PCI layout
- Gate D cross-checks e1000 PCI MMIO against host network interface metadata
