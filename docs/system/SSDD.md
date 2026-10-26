# SSDD — System/Subsystem Design Description

**Product:** Hyperviseur edge gateway (spike)  
**Version:** 0.1 (Task 0 skeleton; as-built sync in Task 11)  
**Language:** English

## Table of contents

1. [Purpose](#1-purpose)
2. [Design overview](#2-design-overview)
3. [Workspace layout](#3-workspace-layout)
4. [Runtime components](#4-runtime-components)
5. [Datapath](#5-datapath)
6. [Configuration and harness](#6-configuration-and-harness)
7. [Verification strategy](#7-verification-strategy)
8. [Trusted computing base](#8-trusted-computing-base)
9. [Error handling](#9-error-handling)
10. [Traceability to SSS](#10-traceability-to-sss)

## 1. Purpose

Describe the intended architecture for the spike. This document tracks the design; Task 11 updates it to match the as-built workspace.

## 2. Design overview

The spike is a **vertical slice**:

1. Cargo workspace + `xtask` harness and config schema  
2. Shared `ring` and `isolation` (Verus)  
3. `pci-vf` with `Sriov` / `Inject` behind one `NicBackend` API  
4. WAN/LAN guests forwarding via ring  
5. L1 `hv` with VT‑x / EPT under nested QEMU  
6. Smoke, coverage, Verus verify, E2E bench  

Guests never hardcode BDFs, ring sizes, or memory maps — they consume a **boot info blob** prepared by `hv` from config.

## 3. Workspace layout

Intended tree (landed in later tasks; N/A until Task 1+):

```text
hyperviseur/                 # repo root (AI-Pervisor)
  Cargo.toml                 # workspace
  rust-toolchain.toml
  config/demo.example.toml
  xtask/                     # std — operator commands
  hv/                        # no_std+alloc — Type-1
  guest-wan/                 # no_std+alloc
  guest-lan/                 # no_std+alloc
  ring/                      # no_std — shared SPSC (Verus)
  isolation/                 # no_std — region/perms (Verus)
  pci-vf/                    # no_std — NicBackend
  crates/demo-config/        # std — TOML + env loader
  docs/TCB.md
  docs/HARNESS.md
  docs/system/               # this suite
```

## 4. Runtime components

### 4.1 Hypervisor `hv`

- VMXON and minimal VMCS per vCPU  
- Handled exits (spike): EPT violation, HLT, I/O UART, MSR whitelist  
- EPT maps: private WAN, private LAN, shared ring (explicit RW)  
- Two vCPUs (one per guest); no overcommit in spike  
- UART emulated via exit for logs  
- Loads guest images and flattened config from paths/values in `DemoConfig`

### 4.2 Guests

| Guest | Role |
|-------|------|
| WAN | Host-facing NIC (VF or Inject); forward to LAN via ring |
| LAN | Host-facing NIC (VF or Inject); forward to WAN via ring |

Both poll rings; use zero-copy descriptors into a shared payload arena. Counters live on a shared **stats** page (see ICD).

### 4.3 `pci-vf`

Single trait `NicBackend` (`rx_ready`, `try_recv`, `try_send`). Backend selected **from config**, not compile-time test switches:

- `Sriov` when BDFs present and required  
- `Inject` when config sets `backend=inject`, or when `allow_inject_fallback` and VF absent  
- `Sriov` required but device missing → **fail fast** (no silent mock)

### 4.4 `ring` / `isolation`

- `ring`: SPSC (or dual) shared ring with Verus contracts (no overflow, modulo indices)  
- `isolation`: `Region` / `MemoryMap::check_access` used by EPT intent and guest asserts

## 5. Datapath

**Smoke:** inject/sriov → WAN → ring → LAN → counters (and reverse). Magic serial string `SMOKE_OK` from LAN path.

**Bench:** 64‑byte frames, bidirectional, duration from config; report Gbps, pps, loss, `target_met`.

Shared ring is the guest↔guest fast path; host↔guest always goes through `NicBackend`.

## 6. Configuration and harness

| Mechanism | Role |
|-----------|------|
| `config/demo.toml` | Runtime demonstrator parameters |
| Env overrides | `HV_DEMO_CONFIG`, `HV_VF_BACKEND`, `HV_WAN_VF_BDF`, `HV_LAN_VF_BDF` |
| `cargo xtask` | `build`, `run-nested`, `verify`, `smoke`, `bench`, `coverage`, `gdb` |

Incomplete config → load/boot failure. Schema ownership: [ICD.md](./ICD.md) `IF-CFG-*`.

## 7. Verification strategy

| Layer | Approach |
|-------|----------|
| Unit | Real ring / Inject / isolation structures in-process — no business mocks |
| Integration | Nested QEMU via `run-nested` / `smoke` |
| Proofs | Verus on annotated safe modules; `verify` fails if proofs break |
| Coverage | `cargo-llvm-cov` via `xtask coverage`; threshold 90%; TCB exclusions listed |
| Bench | Measurement always reported; missing 1 Gbps is not a tool failure |

## 8. Trusted computing base

Thin `unsafe` wrappers (documented in `docs/TCB.md` when code lands):

- VMCS / VT‑x / VMXON programming  
- EPT table build/load (Rust region model is proved; hardware walk is not)  
- SR‑IOV VF MMIO / config space  

Everything else in the datapath and shared structures is a Verus candidate.

## 9. Error handling

| Condition | Behavior |
|-----------|----------|
| Unexpected exit | UART log + halt faulting guest; peer continues if possible |
| Ring full | Drop + counter (no indefinite block) |
| `Sriov` without device | Fail fast |
| Incomplete config | Refuse start |

## 10. Traceability to SSS

| SYS | Design element |
|-----|----------------|
| SYS-001 | EPT maps + `isolation` |
| SYS-002 | Guests + ring + NicBackend |
| SYS-003 | `pci-vf` backend policy |
| SYS-004 | `xtask bench` + stats page |
| SYS-005 | Verus + TCB list |
| SYS-006 | `xtask coverage` |
| SYS-007 | `demo-config` + boot blob |
| SYS-008 | `xtask` CLI |

**Deviations from design:** None recorded at Task 0.
