# SSS — System/Subsystem Specification

**Product:** Hyperviseur edge gateway (spike)  
**Version:** 0.1 (Task 0 skeleton)  
**Language:** English

## Table of contents

1. [Purpose](#1-purpose)
2. [Scope](#2-scope)
3. [References](#3-references)
4. [System context](#4-system-context)
5. [System needs (`SYS-*`)](#5-system-needs-sys-)
6. [Constraints](#6-constraints)
7. [Non-goals](#7-non-goals)
8. [Success criteria](#8-success-criteria)
9. [Assumptions and dependencies](#9-assumptions-and-dependencies)

## 1. Purpose

Specify the system-level needs for a **Type‑1 nested hypervisor** spike that isolates WAN and LAN edge guests, moves packets through a shared guest↔guest ring, and measures bidirectional throughput at 64‑byte frames under a Cargo + `xtask` harness.

## 2. Scope

**In scope for the spike**

- L1 hypervisor (`hv`) owning VT‑x / EPT under QEMU nested (`+vmx`)
- Two Rust `no_std` (+ `alloc`) guests: WAN and LAN
- Shared memory ring for guest↔guest datapath
- Host↔guest I/O via SR‑IOV VF when available, or config-gated `Inject` backend
- Verus proofs on all provable safe Rust; coverage ≥ 90% on declared crates
- Documentary suite (this package)
- Config-driven demonstrator (`config/demo.toml` + env); no lab hardcoding

**Out of scope**

See [§7 Non-goals](#7-non-goals).

## 3. References

| Ref | Document |
|-----|----------|
| R1 | Design: Hyperviseur edge gateway Rust — spike (2026-10-26) |
| R2 | Implementation plan: Hyperviseur edge spike (2026-10-26) |
| R3 | [SSDD.md](./SSDD.md), [SRS.md](./SRS.md), [ICD.md](./ICD.md), [STD.md](./STD.md), [STR.md](./STR.md) |
| R4 | `docs/TCB.md`, `docs/HARNESS.md` (later tasks) |

## 4. System context

```text
                  ┌─────────────────────────────────────┐
  Host / lab      │  QEMU nested (+vmx, IOMMU, VFIO)     │
  traffic         │                                     │
       │          │   ┌────────── L1 hv (VT-x/EPT) ──┐  │
       │          │   │  WAN guest │ ring │ LAN guest │  │
       └──────────┼──►│  (VF/Inj)  │◄───►│ (VF/Inj)  │  │
                  │   └────────────┴──────┴───────────┘  │
                  └─────────────────────────────────────┘
```

Operators drive build/run/verify/smoke/bench/coverage exclusively through `cargo xtask` (no just/make).

## 5. System needs (`SYS-*`)

| ID | Need | Notes |
|----|------|-------|
| SYS-001 | Isolate WAN and LAN private memory via EPT; shared ring pages mapped RW to both guests | Faulting access → clean exit, no silent corruption |
| SYS-002 | Provide bidirectional host↔WAN↔ring↔LAN↔host datapath | Smoke and bench exercise real nested path |
| SYS-003 | Prefer SR‑IOV VF for host↔guest; allow `Inject` only when config permits and VF absent | `Inject` is degraded production backend, not a test mock |
| SYS-004 | Measure E2E Gbps / pps / loss @ 64 B bidirectional | Target 1 Gbps is an objective; report always published |
| SYS-005 | Prove safe datapath / ring / isolation logic with Verus | TCB `unsafe` listed in `docs/TCB.md` |
| SYS-006 | Enforce ≥ 90% line coverage on declared crates | `cargo xtask coverage` fails under threshold |
| SYS-007 | Drive all demonstrator parameters from config + env | Incomplete config → refuse boot |
| SYS-008 | Provide operator harness: build, run-nested, verify, smoke, bench, coverage, gdb | Cargo workspace + `xtask` only |

## 6. Constraints

| Topic | Constraint |
|-------|------------|
| Hypervisor type | Type‑1 (VT‑x / EPT in project code) |
| Initial lab | Nested QEMU/KVM; Intel x86_64 first |
| Guests | Rust `no_std` + `alloc`; no Linux guest in spike |
| Harness | Cargo only + `cargo xtask` |
| Quality | No mocks/stubs for speed; no demo hardcoding |
| Docs | English; home-grown suite under `docs/system/` |

## 7. Non-goals

| Item | Status |
|------|--------|
| Bare-metal Intel boot | N/A for spike (after spike) |
| Linux guests | N/A for spike |
| Hardware EPT walk proofs beyond Rust isolation model | N/A for spike |
| RISC‑V / ARM | N/A for spike |

## 8. Success criteria

1. `cargo xtask smoke` green (two guests, ring echo, counters) — nested, config-driven  
2. `cargo xtask verify` green on all annotated crates  
3. EPT out-of-region access → proper exit (no silent corruption)  
4. `cargo xtask bench` publishes E2E @ 64 B report (`target_met` may be false)  
5. `cargo xtask coverage` ≥ 90% on declared perimeter  
6. No demo hardcoding; example config checked in  
7. VF howto documented so guests stay unchanged when switching backend  
8. Documentary suite complete; STR filled after campaign (Task 11)

## 9. Assumptions and dependencies

| Item | Assumption |
|------|------------|
| Lab CPU | Nested VT‑x available (`+vmx`) |
| Optional VF | IOMMU + VFIO binding when `backend=sriov` |
| Toolchain | Rust nightly as required by Verus / `no_std` targets |
| Verus | Available to `cargo xtask verify` |

**Open issues:** None for Task 0 documentary skeleton.
