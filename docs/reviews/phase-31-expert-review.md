# Phase 31 expert review (virtualization)

Multi-domain review of EPT-aware relay measurement, ABI v2 extension tail, IPC cross-check, and TSC throughput (`cursor/phase-31-measurement-hardening-0b4f`).

**Reviewer lens:** x86 VMX bring-up, EPT GPA translation, guest measurement trust boundaries, throughput attestation.

## Executive summary

Phase 31 closes the four Phase 30 deferrals with structurally correct scaffolding: explicit `GuestBootInfoRelayMeasurement` ABI v2 tail, EPT GPA resolution, IPC queue tail cross-check, and TSC capture in the out guest. Gate D wires programmed EPT tables and passes elapsed TSC into live throughput.

**Verdict:** Approve as incremental hardening. Several semantics gaps remain (see findings below) — address in Phase 32 before treating measurement as production-grade E2E proof.

---

## Findings

| Severity | Finding | Detail |
|----------|---------|--------|
| **High** | Extension counter incremented per loop, not per delivery | `run_out_sustained` called `record_relay_frame_completed` every iteration regardless of IPC dequeue / payload match |
| **High** | IPC read failures silently ignored | `read_ipc_delivered_frames_from_guest(...).unwrap_or(0)` bypassed E2E cross-check when IPC mapping unreadable |
| **Medium** | EPT resolve checked start address only | Multi-byte reads could span past mapping end without error |
| **Medium** | TSC optional on live Executed path | `apply_live_guest_throughput_benchmark` fell back to mock nanos when `elapsed_tsc == 0` |
| **Medium** | IPC cross-check skipped when tail is zero | `end_to_end_relay_frames` ignored IPC when `ipc_delivered_frames == 0`, trusting extension-only count |
| **Low** | ABI v2 tail check used frames field offset | Parser should validate full 32-byte extension via `guest_boot_info_relay_measurement_offset` |
| **Low** | Guest counters still guest-writable | Expected for smoke; hypervisor-owned page deferred |

---

## Domain notes

### EPT-aware reads

`resolve_guest_phys_to_host` plus host copy in `hv-x86-cpu` is the right split (EPT crate stays `unsafe`-free). Reads must validate the **full byte range** lies within one programmed mapping.

### IPC tail semantics

IPC queue `tail` (offset 4) counts consumer dequeues — aligned with out-partition delivery when in/mid guests enqueue successfully. Cross-check requires non-zero IPC tail on `Executed` paths.

### TSC throughput

Out guest RDTSC bracketing is correct. Live throughput must require non-zero elapsed TSC when execution is `Executed` and frame threshold is met; mock fallback is validate-only.

### Trust model (unchanged)

Extension and IPC state remain guest-writable RAM. Phase 31 adds **consistency checks**, not cryptographic attestation.

---

## Verification baseline

- `cargo test -p hv-guest-abi -p hv-ept -p hv-guest-boot`
- `cargo test -p hv-datapath guest_relay`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-relay-measurement`
- `cargo xtask build-guests && cargo xtask build-boot-chain-live`
