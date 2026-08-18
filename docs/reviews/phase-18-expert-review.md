# Phase 18 expert review

Multi-domain review of Gate D datapath live: IPC queue runtime, e1000 MMIO smoke model, synthetic in→mid→out forwarding, VM-exit seam foundation, and `datapath-live` orchestration (`cursor/phase-18-datapath-live-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| IPC queue runtime | `IpcQueueView`, slot metadata, enqueue/dequeue bounds checks |
| e1000 MMIO model | TDT/RDT doorbell registers, TX smoke trigger |
| Datapath forwarding | `plan_datapath_forward`, `forward_synthetic_frame`, `MockDatapathBackend` |
| EPT MMIO mappings | e1000 guest-phys regions added to `plan_ept_init` |
| VM-exit seam | `run_datapath_live_cpu_seam`, validate-only default |
| Gate D live orchestration | `GateDDatapathLiveResult`, `boot_*_gate_d_datapath_live*()` |
| Malicious IPC tests | Host-side oversize payload, empty dequeue, truncated region |
| UEFI + xtask | `datapath-live` feature chain, serial markers, coverage pass |

## Phase 17 deferrals closed

| Phase 17 item | Phase 18 disposition |
|---------------|---------------------|
| Live IPC forwarding | **Partially closed** — synthetic frame in→mid→out in mock runtime |
| e1000 datapath (Gate D) | **Partially closed** — MMIO doorbell + EPT mapping; live guest TX/RX deferred |
| VM-exit dispatch | **Partially closed** — seam validates stub address; handler body deferred |
| Malicious guest tests | **Partially closed** — host-side IPC integrity tests only |
| 200 Mbit/s performance benchmark | **Deferred** |
| Multi-partition VMLAUNCH | **Deferred** |
| Real guest ELF images | **Deferred** |
| DMAR MMIO | **Unchanged** |

## Feature matrix

| Feature | Crate | Default | UEFI chain | Effect |
|---------|-------|---------|------------|--------|
| `datapath-live` | `hv-hypervisor-boot` | off | off | Gate D live orchestration atop `datapath-foundation` |
| `datapath-live` | `hv-hypervisor-efi` | off | opt-in | Live boot entry + IPC/e1000 markers |
| `datapath-live` | `hv-hypervisor-efi-bin` | off | opt-in | Builds on `datapath-foundation` |
| `datapath-live` | `hv-x86-cpu` | off | off | VM-exit stub seam validation |

## Verification

- `cargo test --workspace` — pass
- `cargo test -p hv-hypervisor-boot --features datapath-live` — pass
- `cargo test -p hv-hypervisor-efi --features datapath-live` — pass
- `cargo test -p hv-datapath` — pass (includes `malicious_ipc`)
- `cargo clippy --all-targets --all-features -- -D warnings` — pass
- `cargo xtask coverage` — multi-pass includes `datapath-live`
- `cargo xtask build-boot-chain-live` — builds with `real-hw-execution,vmx-launch,datapath-foundation,datapath-live`

## Review status

Phase 18 closes the live datapath scaffolding gap from Phase 17: IPC queue operations, e1000 MMIO smoke, synthetic three-hop forwarding, and Gate D live orchestration with validate-only CPU seams. Full guest runtime, compromised-partition malicious tests, and performance benchmarking remain deferred to Phase 19+.
