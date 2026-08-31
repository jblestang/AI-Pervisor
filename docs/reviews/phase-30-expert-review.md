# Phase 30 expert review

Addresses Phase 29 expert review findings (`cursor/phase-30-review-fixes-0b4f`).

## Fixes applied

| Review finding | Fix |
|----------------|-----|
| No VM-exit / VMRESUME loop | VM-exit stub installed at `HOST_RIP`; `vmlaunch_wait_for_hlt_exit` resumes on `HLT` via stub `ret` + `VMRESUME` otherwise |
| Measurement reads GPA as HPA | Documented identity contract on `GuestBootInfoMeasurementSite`; non-zero address validation |
| Per-partition counters ≠ E2E | Read **out-partition only** (`VmId` 2); counter incremented only in `out` guest sustained loop |
| ABI tail without version bump | `GUEST_ABI_VERSION = 2`; compatibility accepts v1–v2; parse validates v2 tail |
| `live_relay_validated` misnamed | Tied to `in_vm_relay_frames >= expected` when measurement feature enabled |
| Guest-writable counter on all partitions | Counter updates restricted to out-partition relay completions |

## Deferred (unchanged)

| Item | Reason |
|------|--------|
| EPT-aware GPA→HPA read | Requires non-identity EPT bring-up (future phase) |
| Wall-clock in-VM throughput | Still uses mock timing budget |
| Full attestation of guest counters | Smoke scaffolding; out-partition + IPC validation deferred |

## Verification

- `cargo test -p hv-guest-abi`
- `cargo test -p hv-guest-boot`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-relay-measurement`
- `cargo xtask build-boot-chain-live`
