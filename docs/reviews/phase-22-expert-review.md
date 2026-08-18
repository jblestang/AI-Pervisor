# Phase 22 expert review

Multi-domain review of Gate D datapath runtime: guest-driven in→mid→out traversal, datapath-capable guest ELFs, multi-partition VM-exit seam validation, and `datapath-runtime` feature chain (`cursor/phase-22-datapath-runtime-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Guest runtime backend | `run_guest_datapath_runtime`, `GuestDatapathRuntime`, per-partition IPC/e1000 hops |
| Datapath guest ELFs | `GUEST_{IN,MID,OUT}_DATAPATH_ELF`, `reference_datapath_guest_elf` |
| Multi-partition runtime seam | `run_datapath_runtime_cpu_seam`, VM-exit stub validation |
| Gate D runtime orchestration | `GateDDatapathRuntimeResult`, `boot_*_gate_d_datapath_runtime*()` |
| UEFI + xtask | `datapath-runtime` feature chain, serial markers, coverage pass |

## Phase 21 deferrals closed

| Phase 21 item | Phase 22 disposition |
|---------------|---------------------|
| Live guest datapath runtime under VMX | **Partially closed** — guest-role hops + multi-partition runtime seam (validate-only default) |
| Real `guests/` source trees | **Unchanged** |
| Live 200 Mbit/s under VMX | **Unchanged** — mock benchmark retained |

## Feature matrix

| Feature | Crate | Default | UEFI chain | Effect |
|---------|-------|---------|------------|--------|
| `datapath-runtime` | `hv-hypervisor-boot` | off | off | Gate D runtime orchestration atop `datapath-benchmark` |
| `datapath-runtime` | `hv-x86-cpu` | off | off | Multi-partition datapath runtime CPU seam |
| `datapath-runtime` | `hv-hypervisor-efi` | off | opt-in | Runtime boot entry + guest frame marker |
| `datapath-runtime` | `hv-hypervisor-efi-bin` | off | opt-in | Builds on `datapath-benchmark` |

## Serial markers

- `GATE_D_DATAPATH_RUNTIME_MARKER` — runtime orchestration succeeded
- `GATE_D_GUEST_DATAPATH_FRAME_MARKER` — guest-driven in→mid→out frame observed

## Verification

- `cargo test --workspace` — pass
- `cargo test -p hv-datapath` — pass (guest runtime fixtures)
- `cargo test -p hv-hypervisor-boot --features datapath-runtime` — pass
- `cargo test -p hv-hypervisor-efi --features datapath-runtime` — pass
- `cargo clippy --all-targets --all-features -- -D warnings` — pass
- `cargo xtask coverage` — multi-pass includes `datapath-runtime`
- `cargo xtask build-boot-chain-live` — builds with full `datapath-*` chain including `datapath-runtime`

## Review status

Phase 22 closes the live guest datapath runtime scaffolding gap from Phase 21: datapath-capable reference ELFs, guest-role IPC/e1000 traversal, multi-partition VM-exit seam validation, and Gate D runtime orchestration with serial markers. Real guest source trees and live VMX throughput benchmarking remain deferred to Phase 23+.
