# Phase 43 expert review

VM-exit-driven per-frame relay counter increment (`cursor/phase-43-vmexit-frame-counter-0b4f`).

## Changes

| Component | Change |
|-----------|--------|
| VM-exit dispatch | Ret-only stub + host-side `vmlaunch_to_host` / `vmresume_to_host` loop |
| Relay counter | `VmexitRelayCounterConfig`; increment on EPT violation write to measurement page GPA |
| Execution seam | `vmexit_relay_frames` on `DatapathGuestExecutionCpuSeamOutcome` |
| Publish | Authoritative frame count from VM-exit increments; IPC cross-check retained |
| Guest firmware | Out partition signals frames via read-only measurement page write (EPT trap) |
| Gate D | Pass relay counter config; reject missing/mismatched VM-exit frame counts |

## Trust model

Frame counts now originate from hypervisor-side VM-exit handling on read-only measurement page writes instead of IPC-derived publish-time counts or guest-writable boot-info tails. IPC delivered tail remains a conservative cross-check. Full live VM-exit/resume relay loop for IPC/MMIO datapath remains deferred.

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-relay-measurement`
- `cargo xtask build-guests && cargo xtask build-boot-chain-live`
