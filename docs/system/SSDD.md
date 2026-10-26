# System / Subsystem Design Description (SSDD)

| Field | Value |
|-------|-------|
| **Document ID** | HV-EDGE-SSDD-001 |
| **Title** | Hypervisor Edge Gateway — System / Subsystem Design Description |
| **Issue** | 0.2 |
| **Date** | 2026-10-26 |
| **Classification** | UNCLASSIFIED — Technology Demonstrator |
| **Parent suite** | HV-EDGE-DOC-SUITE |
| **Implements** | HV-EDGE-SSS-001 Issue 0.2 |
| **As-built sync** | Partial (workspace Tasks 1–3 artefacts present); full sync at Task 11 |

---

## Table of contents

1. [Document control](#1-document-control)  
2. [Design purpose and drivers](#2-design-purpose-and-drivers)  
3. [Architectural overview](#3-architectural-overview)  
4. [Hardware / platform allocation](#4-hardware--platform-allocation)  
5. [Software architecture](#5-software-architecture)  
6. [Modes of operation](#6-modes-of-operation)  
7. [Datapath design](#7-datapath-design)  
8. [Trusted Computing Base](#8-trusted-computing-base)  
9. [Error handling and degraded modes](#9-error-handling-and-degraded-modes)  
10. [Security and isolation design](#10-security-and-isolation-design)  
11. [Assurance design (proofs and coverage)](#11-assurance-design-proofs-and-coverage)  
12. [Traceability SSS → design](#12-traceability-sss--design)  
13. [Open design points](#13-open-design-points)  
14. [Change record](#14-change-record)

---

## 1. Document control

This SSDD describes **how** the system of interest realises SSS needs. Normative interface layouts live in HV-EDGE-ICD-001. Normative software shalls live in HV-EDGE-SRS-001.

**As-built policy:** When implementation diverges, Task 11 updates this SSDD; until then, deviations are listed in §13.

---

## 2. Design purpose and drivers

| Driver | Design response |
|--------|-----------------|
| Strong domain isolation | VT‑x guests + EPT private maps; shared only what ICD names |
| High-rate small frames | Poll-mode guests; SPSC shared ring; zero-copy descriptors |
| Lab realism without fake tests | Single `NicBackend`; Sriov preferred; Inject explicit and config-gated |
| Demonstrator reproducibility | All parameters from TOML + env; refuse incomplete config |
| Assurance under schedule | Verus on safe crates; thin unsafe TCB; coverage gate ≥ 90% |
| Operator efficiency | Single harness: `cargo xtask …` |

---

## 3. Architectural overview

### 3.1 Layering

```text
  ┌─────────────────────────────────────────────────────────┐
  │  Operator plane (std)                                   │
  │  xtask · demo-config · QEMU launch · reports            │
  └───────────────────────────┬─────────────────────────────┘
                              │ images + config → nested VM
  ┌───────────────────────────▼─────────────────────────────┐
  │  L1 Hypervisor plane (no_std+alloc)                     │
  │  hv: VMXON/VMCS · EPT · guest load · UART · boot blob   │
  └───────────────┬─────────────────────────────┬───────────┘
                  │                             │
  ┌───────────────▼──────────┐   ┌──────────────▼───────────┐
  │  WAN guest (no_std+alloc)│   │  LAN guest (no_std+alloc)│
  │  NicBackend · forward    │◄─►│  NicBackend · forward    │
  │                          │ring│                         │
  └──────────────────────────┘   └──────────────────────────┘
                  │ shared crates: ring · isolation · pci-vf
```

### 3.2 Design principles

1. **Config over literals** — no lab BDF/size/duration constants in guest/`hv` source.  
2. **One NIC API** — guests unaware of Sriov vs Inject.  
3. **Prove the model, list the TCB** — isolation/ring safe logic Verus-candidate; VMX/EPT walk hardware residual.  
4. **Fail loud** — missing Sriov device or incomplete config aborts; ring full drops with counter.

---

## 4. Hardware / platform allocation

| Function | Allocation | Notes |
|----------|------------|-------|
| CPU virtualisation (VMX) | HW (Intel VT‑x) + SW (`hv`) | Nested under KVM for demonstrator |
| Memory isolation | HW (EPT) + SW (map builder + `isolation` model) | Model proved; walk in TCB |
| DMA isolation (VF) | HW (IOMMU) + L0 VFIO | Required for official Sriov path |
| Host↔guest frames | HW VF **or** SW Inject queue | Selected by config |
| Guest↔guest frames | SW shared memory ring | Coherent pages via EPT |
| Time / bench duration | SW harness + guest counters | Host clock for duration wall |
| Console / evidence | HW serial via QEMU + SW UART exit | `SMOKE_OK` protocol |

**Laboratory reference platform (assumption ASM-LAB-*):** Intel-class host capable of nested VT‑x; representative target cited in design: 8 cores / 32 GiB / IOMMU. Demonstrator QEMU memory/SMP come from `[lab]` config, not hardcoded.

---

## 5. Software architecture

### 5.1 Workspace packages (intended / partial as-built)

| Package | Std? | Responsibility | SSS allocation |
|---------|------|----------------|----------------|
| `xtask` | std | Operator CLI: build, run-nested, verify, smoke, bench, coverage, gdb | SYS-AS-004 |
| `demo-config` | std | Parse/validate TOML + env → `DemoConfig` | SYS-AS-003 |
| `hv` | no_std+alloc | Type‑1: VMX, EPT, load guests, boot blob, UART | SYS-ISO-*, SYS-DP-* |
| `guest-wan` / `guest-lan` | no_std+alloc | Domain datapath | SYS-DP-001 |
| `ring` | no_std | SPSC shared ring (`FrameDesc`) | SYS-DP-001 |
| `isolation` | no_std | Region/permission model | SYS-ISO-004 |
| `pci-vf` | no_std | `NicBackend` Sriov / Inject | SYS-DP-002..004 |

### 5.2 Hypervisor (`hv`) — logical modules

| Module (logical) | Duty |
|------------------|------|
| VMX bring-up | VMXON, VMCS per vCPU |
| Exit handling | EPT violation, HLT, UART I/O, MSR whitelist |
| EPT builder | Apply `isolation::MemoryMap` intent to tables |
| Guest memory | Allocate private + shared regions from config sizes |
| Loader | Load guest images from `paths.*` |
| Boot info | Fill ICD `IF-BOOT-*` blob |
| UART emu | Exit → serial |

**vCPU topology (demonstrator):** one vCPU per guest; no overcommit.

### 5.3 Guests

| Guest | Host face | Peer face |
|-------|-----------|-----------|
| WAN | `NicBackend` (VF or Inject) | Produce/consume ring toward LAN |
| LAN | `NicBackend` (VF or Inject) | Produce/consume ring toward WAN |

Forward logic is shared where practical (`guest-common` when introduced) so unit tests exercise the **same** code guests link — without mocks of the ring or Inject queue.

### 5.4 Configuration flow

```text
demo.toml + env  →  demo-config::DemoConfig
                         │
          ┌──────────────┼──────────────────┐
          ▼              ▼                  ▼
       xtask QEMU     flattened          boot info
       argv only      NIC config         blob in guest
       from config    for pci-vf open    (IF-BOOT-*)
```

---

## 6. Modes of operation

| Mode | Entry | Behaviour | Exit |
|------|-------|-----------|------|
| **BUILD** | `cargo xtask build` | Produce `hv` / guest images | Artefacts on disk |
| **NESTED_RUN** | `run-nested` | Boot L1+guests per config | Operator stop / panic |
| **SMOKE** | `smoke` | Nested run until `SMOKE_OK` or timeout | Pass/fail to STD |
| **BENCH** | `bench` | Timed bidirectional load; write JSON | Report always if measured |
| **VERIFY** | `verify` | Verus on annotated crates | Pass/fail per VERUS.md |
| **COVER** | `coverage` | llvm-cov; fail &lt; 90% | Pass/fail |
| **DEGRADED_INJECT** | config permits Inject | Same guest code; SW queue I/O | Same smoke protocol |
| **FAULT_GUEST** | EPT violation / unexpected exit | Log + halt faulting guest; peer may continue | Manual reset |

---

## 7. Datapath design

### 7.1 Smoke path

```text
Inject/Sriov RX → WAN guest → try_push(ring) → LAN try_pop → NicBackend TX
(+ reverse direction as configured)
→ stats++ → UART "SMOKE_OK"
```

### 7.2 Ring

- Layout and concurrency: ICD `IF-RING-*` (header + `FrameDesc` slots; free-running indices; power-of-two capacity).  
- Full: producer drops + `ring_drop_full` (no indefinite block).  
- Payload in separate shared arena; descriptors carry `offset`/`len` only.

### 7.3 Bench path

Harness drives load per `[bench]`; samples stats; writes report (`Gbps`, `pps`, `loss`, `duration`, `target_met`). Exit code 0 if measurement completed; target miss is **not** a tool failure (SYS-DP-006).

---

## 8. Trusted Computing Base

| TCB class | Examples | Proof status |
|-----------|----------|--------------|
| VMX control | VMXON, VMCS R/W | Unproved — listed in `docs/TCB.md` |
| EPT hardware walk | Processor use of EPT tables | Unproved; **model** in `isolation` is Verus-candidate |
| VF MMIO | Sriov config/MMIO | Unproved thin wrappers |
| Shared-memory atomics correctness vs HW coherency | Mapping/coherency by `hv` | Protocol in ICD; mapping in TCB |

All other datapath and shared structures are **assurance candidates** (SYS-AS-001).

---

## 9. Error handling and degraded modes

| Condition | System response | Requirement |
|-----------|-----------------|-------------|
| Incomplete config | Refuse start; name missing fields | SYS-AS-003 |
| Sriov required, VF missing | Fail fast | SYS-DP-004 |
| Ring full | Drop + counter | SRS SWR-DP-013 |
| Unexpected VM exit | UART log; halt faulting guest | SYS-ISO-003 |
| Peer guest halt | Remaining guest continues if possible | Design goal |
| Bench target miss | Valid report, `target_met=false` | SYS-DP-006 |

---

## 10. Security and isolation design

**Objectives (demonstrator):** spatial isolation of WAN/LAN private memory; explicit shared pages; no silent cross-domain corruption on fault.

**Non-objectives:** cryptographic separation, covert-channel analysis, attested boot, supply-chain hardening beyond normal repo practice.

**Controls:**

| Control | Mechanism |
|---------|-----------|
| Domain memory isolation | EPT permissions from `isolation` map |
| Shared-memory minimisation | Only ring header/slots, arena, stats (ICD) |
| Config integrity | Schema validation before launch |
| Backend honesty | No silent Sriov→mock substitution |

---

## 11. Assurance design (proofs and coverage)

| Layer | Mechanism | Gate |
|-------|-----------|------|
| Unit | Host tests on real ring / isolation / Inject | STD unit procedures |
| Integration | Nested QEMU smoke | TEST-SMOKE-001 |
| Proof | Verus on annotated crates; see `docs/VERUS.md` | TEST-VERUS-001 |
| Coverage | `cargo xtask coverage`, threshold 90%, allowlist for TCB only | TEST-COV-001 |
| Measurement | Bench JSON | TEST-BENCH-001 |

---

## 12. Traceability SSS → design

| SYS ID | Design element |
|--------|----------------|
| SYS-ISO-001..003 | `hv` EPT + exit path |
| SYS-ISO-004 | `isolation` crate |
| SYS-DP-001 | guests + `ring` |
| SYS-DP-002..004 | `pci-vf` + config policy |
| SYS-DP-005..006 | stats + `xtask bench` |
| SYS-AS-001..002 | Verus + coverage |
| SYS-AS-003..004 | `demo-config` + `xtask` |
| SYS-AS-005..006 | `docs/TCB.md` + `docs/system/` |

---

## 13. Open design points

| ID | Topic | Status | Resolution target |
|----|-------|--------|-------------------|
| ODP-001 | Exact `IF-BOOT-*` `#[repr(C)]` byte layout | Semantics fixed in ICD; packing at guest bring-up | Task 5–6 |
| ODP-002 | Dual-ring vs two unidirectional SPSC instances | Dual logical directions required; packing choice open | Task 5 |
| ODP-003 | Full `verus!`/`vstd` vs stubs | Stubs present; hard-fail proofs | Task 8 |
| ODP-004 | Automated EPT-violation stimulus | Manual acceptable until harnessed | Task 6–7 / STR |

**Known as-built (Issue 0.2):** workspace, `demo-config`, `xtask` skeleton, `ring` SPSC with host tests, `isolation` model with host tests, Verus stub macros, toolchain 1.99.1 — see repository and `docs/VERUS.md`.

---

## 14. Change record

| Issue | Date | Description |
|-------|------|-------------|
| 0.1 | 2026-10-26 | Skeleton |
| 0.2 | 2026-10-26 | Full SSDD: allocation, modes, TCB, security, assurance, ODPs |
