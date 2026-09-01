# Phase 47 expert review

VM-exit/resume relay loop for IPC shared-memory datapath (`cursor/phase-47-vmexit-ipc-relay-0b4f`).

## Changes

| Component | Change |
|-----------|--------|
| IPC relay | `VmexitIpcRelayConfig`; EPT write trap + hypervisor-side byte relay into queue backing |
| Queue init | `initialize_ipc_queue_backing()` seeds reference IPC header before guest execution |
| Dispatch | `VmexitRelayDispatchPlan` extended with per-VM IPC configs alongside MMIO and measurement |
| Guest run loop | VM-exit handler reads `GUEST_RAX` and instruction length to relay trapped IPC stores |
| Execution seam | `vmexit_ipc_relay_events` on `DatapathGuestExecutionCpuSeamOutcome` |
| Gate D | Read-only IPC EPT traps, backing init, INVEPT reload, executed-path IPC event validation |

## Trust model

Guest IPC shared-memory writes now trap via read-only EPT mappings; the hypervisor relays store data into hypervisor-owned queue backing on VM-exit instead of allowing direct guest writes. Guest reads continue from the same read-only EPT mapping so producer/consumer visibility stays consistent. MMIO relay (Phase 45) and measurement-page frame counting (Phase 43) remain the other VM-exit relay slices.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement`
- `cargo test -p hv-ept`
- `cargo xtask build-guests && cargo xtask build-boot-chain-live`
