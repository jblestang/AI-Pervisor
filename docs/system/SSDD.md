# System / Subsystem Design Description (SSDD)

| Field | Value |
|-------|-------|
| **Document ID** | HV-EDGE-SSDD-001 |
| **Title** | Hypervisor Edge Gateway — System / Subsystem Design Description |
| **Issue** | 0.4 |
| **Date** | 2026-10-26 |
| **Classification** | UNCLASSIFIED — Technology Demonstrator |
| **Parent suite** | HV-EDGE-DOC-SUITE |
| **Implements** | HV-EDGE-SSS-001 Issue 0.3 |
| **As-built sync** | **Complete** (Task 11) — matches workspace SHA recorded in STR |
| **Transfer context** | Safety-related; airborne rates & partitioning (not certified) |

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
10. [Security, isolation, and safety partitioning](#10-security-isolation-and-safety-partitioning)  
11. [Failure modes (FMEA-lite)](#11-failure-modes-fmea-lite)  
12. [Assurance design (proofs and coverage)](#12-assurance-design-proofs-and-coverage)  
13. [Traceability SSS → design](#13-traceability-sss--design)  
14. [As-built status and open design points](#14-as-built-status-and-open-design-points)  
15. [Change record](#15-change-record)

---

## 1. Document control

This SSDD describes **how** the system of interest realises SSS needs. Normative interface layouts live in HV-EDGE-ICD-001. Normative software shalls live in HV-EDGE-SRS-001.

**As-built policy:** Issue 0.4 synchronises this document to the spike workspace after Tasks 0–10. Verification outcomes are recorded in HV-EDGE-STR-001 Issue 0.4. Remaining gaps are listed in §14 and `docs/HARNESS.md` (Task 10).

**Certification claim:** **None** — not DAL/SIL/ASIL/airworthiness approved.

---

## 2. Design purpose and drivers

| Driver | Design response |
|--------|-----------------|
| Strong domain isolation | VT‑x guests + EPT private maps; shared only what ICD names |
| High-rate small frames | Poll-mode guests; SPSC shared ring; zero-copy descriptors |
| Lab realism without fake tests | Single `NicBackend`; Sriov preferred; Inject explicit and config-gated |
| Demonstrator reproducibility | All parameters from TOML + env; refuse incomplete config |
| Assurance under schedule | Verus on `ring` / `isolation` proofs; thin unsafe TCB; coverage gate ≥ 90% |
| Operator efficiency | Single harness: `cargo xtask …` |
| Safety / airborne transfer | Spatial partitioning via EPT; fail-loud exits; published rates; explicit TCB; no false integrity-level claim |

---

## 3. Architectural overview

### 3.1 Layering (as-built)

```text
  ┌─────────────────────────────────────────────────────────┐
  │  Operator plane (std)                                   │
  │  xtask · demo-config · QEMU Multiboot · llvm-cov/Verus  │
  │  reports: SMOKE_OK serial · bench-report.json           │
  └───────────────────────────┬─────────────────────────────┘
                              │ hv.bin + flat blob + guest images
  ┌───────────────────────────▼─────────────────────────────┐
  │  L1 Hypervisor plane (no_std+alloc)                     │
  │  hv: Multiboot · FlatHvConfig · EPT pack · VMX scaffold │
  │  L1 Inject smoke path (guest_common forward → SMOKE_OK) │
  └───────────────┬─────────────────────────────┬───────────┘
                  │                             │
  ┌───────────────▼──────────┐   ┌──────────────▼───────────┐
  │  WAN guest (no_std+alloc)│   │  LAN guest (no_std+alloc)│
  │  NicBackend · forward    │◄─►│  NicBackend · forward    │
  │  (L2 entry incomplete)   │ring│  (L2 entry incomplete)  │
  └──────────────────────────┘   └──────────────────────────┘
        shared: ring · isolation · pci-vf · guest-common
```

### 3.2 Design principles

1. **Config over literals** — no lab BDF/size/duration constants in guest/`hv` source.  
2. **One NIC API** — guests unaware of Sriov vs Inject.  
3. **Prove the model, list the TCB** — isolation/ring lemmas Verus-checked; VMX/EPT walk hardware residual.  
4. **Fail loud** — missing Sriov device or incomplete config aborts; ring full drops with counter; Sriov E2E does not silently become Inject.

---

## 4. Hardware / platform allocation

| Function | Allocation | Notes |
|----------|------------|-------|
| CPU virtualisation (VMX) | HW (Intel VT‑x) + SW (`hv`) | Nested under KVM intended; smoke default is **TCG** Multiboot (see HARNESS) |
| Memory isolation | HW (EPT) + SW (map builder + `isolation` model) | Model proved; walk in TCB |
| DMA isolation (VF) | HW (IOMMU) + L0 VFIO | Required for official Sriov path |
| Host↔guest frames | HW VF **or** SW Inject queue | Selected by config; L1 Inject used for smoke/bench today |
| Guest↔guest frames | SW shared memory ring | Same `SpscRing` in L1 inject + guest-common |
| Time / bench duration | SW harness + guest counters | Host clock for duration wall |
| Console / evidence | HW serial via QEMU + SW UART | `SMOKE_OK` protocol (`IF-SER-001`) |

**Laboratory reference platform (assumption ASM-LAB-*):** Intel-class host capable of nested VT‑x; representative target cited in design: 8 cores / 32 GiB / IOMMU. Demonstrator QEMU memory/SMP come from `[lab]` config, not hardcoded.

---

## 5. Software architecture

### 5.1 Workspace packages (as-built)

| Package | Path | Std? | Responsibility | SSS allocation |
|---------|------|------|----------------|----------------|
| `xtask` | `xtask/` | std | Operator CLI: `build`, `run-nested`, `smoke`, `bench`, `verify`, `coverage` (`gdb` stub) | SYS-AS-004 |
| `demo-config` | `crates/demo-config/` | std | Parse/validate TOML + env → `DemoConfig` | SYS-AS-003 |
| `guest-common` | `crates/guest-common/` | no_std+alloc | BootInfo, forward, stats, UART helpers (shared by guests + L1 smoke) | SYS-DP-001 |
| `hv` | `hv/` | no_std+alloc | Type‑1: Multiboot, EPT, VMX scaffold, boot blob, L1 Inject smoke | SYS-ISO-*, SYS-DP-* |
| `guest-wan` / `guest-lan` | `guest-wan/`, `guest-lan/` | no_std+alloc | Domain entry + NicBackend open from BootInfo | SYS-DP-001 |
| `ring` | `ring/` | no_std | SPSC shared ring (`FrameDesc`) + Verus proofs | SYS-DP-001 |
| `isolation` | `isolation/` | no_std | Region/permission model + Verus proofs | SYS-ISO-004 |
| `pci-vf` | `pci-vf/` | no_std | `NicBackend` Sriov / Inject | SYS-DP-002..004 |

Toolchain: Rust **1.99.1** (`rust-toolchain.toml`), aligned with Verus `0.2026.10.26` (`docs/VERUS.md`).

### 5.2 Hypervisor (`hv`) — as-built modules

| Module | Path | Duty |
|--------|------|------|
| Multiboot / boot | `multiboot.rs`, `boot.s` | Multiboot1 ADDR trampoline → `rust_entry` |
| Flat config | `config.rs` | Parse Multiboot module 0 (`FLAT` blob from xtask) |
| Guest memory layout | `guest_mem.rs` | Private + shared GPA packing from config |
| Guest load | `guest_load.rs` | Copy guest image modules into private buffers |
| Boot info | `boot_blob.rs` | Fill ICD `IF-BOOT-*` blob |
| EPT intent / tables / install | `ept.rs`, `ept_tables.rs`, `ept_install.rs` | Apply `isolation` intent; pack tables |
| VMX | `vmx/` (`vmxon`, `vmcs`, `exit`) | VMXON / VMCS scaffold; exit → UART / halt |
| L1 Inject smoke | `smoke_l1.rs` | Real `guest_common` forward + `SMOKE_OK` when Inject |
| UART | `uart.rs` | Exit / platform serial |

**vCPU topology (demonstrator):** one vCPU per guest; no overcommit.  
**L2 status:** VMLAUNCH / complete VMCS host-state still incomplete — guests are loaded and EPT-packed; live L2 poll is not yet the smoke/bench path (HARNESS Task 10 gaps).

### 5.3 Guests and shared forward

| Guest | Host face | Peer face |
|-------|-----------|-----------|
| WAN | `NicBackend` (VF or Inject) | Produce/consume ring toward LAN |
| LAN | `NicBackend` (VF or Inject) | Produce/consume ring toward WAN |

Forward logic lives in `guest-common` so host unit tests and L1 Inject smoke exercise the **same** code guests link — without mocks of the ring or Inject queue.

### 5.4 Configuration flow (as-built)

```text
demo.toml + env  →  demo-config::DemoConfig
                         │
          ┌──────────────┼──────────────────┐
          ▼              ▼                  ▼
       xtask QEMU     FlatHvConfigBlob   BootInfo in guest
       argv only      (Multiboot mod 0)  (IF-BOOT-*; nic_backend
       from config    + guest modules    follows resolved backend)
```

Resolution policy: Sriov requires VF sysfs **and** `vfio-pci` bind; `allow_inject_fallback` never silently labels Inject as Sriov (`docs/HARNESS.md`).

---

## 6. Modes of operation

| Mode | Entry | Behaviour | Exit |
|------|-------|-----------|------|
| **BUILD** | `cargo xtask build` | Host libs + freestanding `hv`/`guest-*`; objcopy → `hv.bin` | Artefacts on disk |
| **NESTED_RUN** | `run-nested` | Multiboot QEMU; argv from config | Operator stop / panic |
| **SMOKE** | `smoke` | Inject: wait for `SMOKE_OK`; Sriov+VF: fail-fast until L2 ready | Pass/fail to STD |
| **BENCH** | `bench` | Inject harness timed load + JSON; Sriov+VF: fail-fast until L2 | Report if measured |
| **VERIFY** | `verify` | Verus on `ring` + `isolation` | Pass/fail per VERUS.md |
| **COVER** | `coverage` | llvm-cov on allowlisted packages; fail &lt; 90% | Pass/fail |
| **DEGRADED_INJECT** | config permits Inject | Same guest_common code; SW queue I/O | Same smoke protocol |
| **FAULT_GUEST** | EPT violation / unexpected exit | Log + halt faulting guest (policy tested on host) | Manual reset |

---

## 7. Datapath design

### 7.1 Smoke path (as-built Inject)

```text
InjectNic RX → guest_common::forward_wan_poll → try_push(ring)
  → forward_lan_poll → InjectNic TX (+ reverse when bidirectional)
  → stats++ → UART "SMOKE_OK"
```

Executed in L1 (`hv::smoke_l1`) under Multiboot/TCG when resolved backend is Inject. Sriov-resolved smoke does **not** substitute this path.

### 7.2 Ring

- Layout and concurrency: ICD `IF-RING-*` (header + `FrameDesc` slots; free-running indices; power-of-two capacity).  
- Full: producer drops + `ring_drop_full` (no indefinite block).  
- Payload in separate shared arena; descriptors carry `offset`/`len` only.  
- Verus: `ring/src/proofs.rs` (pow2 / mask lemmas).

### 7.3 Bench path (as-built Inject)

`cargo xtask bench` drives the same `guest_common` forward path with Inject + `SpscRing` for `bench.duration_secs` at `bench.frame_size`. Writes `IF-RPT-BENCH` JSON (`Gbps`, `pps`, `loss`, `duration`, `target_met`, `backend`, `environment_class`). Exit code 0 if measurement completed; target miss is **not** a tool failure (SYS-DP-006). Official Sriov figures remain blocked until L2 VMLAUNCH + VF MMIO (HARNESS).

---

## 8. Trusted Computing Base

| TCB class | Examples | Proof status |
|-----------|----------|--------------|
| VMX control | VMXON, VMCS R/W | Unproved — `docs/TCB.md` |
| EPT hardware walk | Processor use of EPT tables | Unproved; **model** in `isolation` Verus-checked |
| VF MMIO | Sriov config/MMIO | Unproved thin wrappers (pre-MMIO stub as-built) |
| Shared-memory atomics vs HW coherency | Mapping by `hv` | Protocol in ICD; mapping in TCB |
| Guest UART / bump alloc | `guest-common` | Listed TCB; coverage-excluded with justification |

All other datapath and shared structures are **assurance candidates** (SYS-AS-001). Normative inventory: `docs/TCB.md`.

---

## 9. Error handling and degraded modes

| Condition | System response | Requirement |
|-----------|-----------------|-------------|
| Incomplete config | Refuse start; name missing fields | SYS-AS-003 |
| Sriov required, VF missing/unbound | Fail fast | SYS-DP-004 |
| Sriov resolved but L2 incomplete | Smoke/bench exit non-zero; no Inject relabel | HARNESS Task 10 |
| Ring full | Drop + counter | SRS SWR-DP-013 |
| Unexpected VM exit | UART log; halt faulting guest | SYS-ISO-003 |
| Peer guest halt | Remaining guest continues if possible | Design goal |
| Bench target miss | Valid report, `target_met=false` | SYS-DP-006 |

---

## 10. Security, isolation, and safety partitioning

**Objectives (demonstrator):** spatial isolation of WAN/LAN private memory; explicit shared pages; no silent cross-domain corruption on fault; behaviour compatible with later **safety-related** or **airborne** partitioning arguments (SG-ISO-*).

**Non-objectives:** cryptographic separation, covert-channel analysis, attested boot, WCET proof, DAL/SIL assignment, supply-chain hardening beyond normal repo practice.

### 10.1 Partitioning model (safety-oriented view)

| Partition | Contents | Trust relative to peer |
|-----------|----------|------------------------|
| WAN guest | Private memory, WAN-facing NIC logic | Must not access LAN private |
| LAN guest | Private memory, LAN-facing NIC logic | Must not access WAN private |
| Shared datapath | Ring header/slots, payload arena, stats | **Intentional** coupling — minimise and ICD-control |
| Hypervisor | Multiboot/EPT/VMX/UART/boot/L1 smoke | TCB relative to guests |

The shared ring is a **deliberate common resource** (RSK-006). Safety transfer programmes must treat it as a coupling surface: capacity, drop policy, and mapping rights are safety-relevant.

**Controls:**

| Control | Mechanism | Safety goal |
|---------|-----------|-------------|
| Domain memory isolation | EPT permissions from `isolation` map | SG-ISO-001 |
| Shared-memory minimisation | Only ring header/slots, arena, stats (ICD) | SG-ISO-002 |
| Violation fail-loud | UART + halt faulting guest | SG-FCC-001 |
| Config integrity | Schema validation before launch | SG-CFG-001 |
| Backend honesty | No silent Sriov→Inject substitution | SG-CFG-001 |
| Rate honesty | Bench JSON with backend/env metadata | SG-RATE-001 |

---

## 11. Failure modes (FMEA-lite)

| ID | Failure mode | Local effect | System effect | Detection | Response | HAZ link |
|----|--------------|--------------|---------------|-----------|----------|----------|
| FM-001 | EPT mis-map shared→private | Peer memory exposed | Cross-domain corruption | Isolation unit tests; ISO-001 host | Halt / refuse boot if detectable at map build | HAZ-MEM-001 |
| FM-002 | Guest writes outside region | EPT violation | Faulting guest stop | Exit handler (host-tested) | Diagnostic + halt faulting guest | HAZ-MEM-001, SG-FCC-001 |
| FM-003 | Ring full under load | Push fails | Frame drop, counter++ | Stats / bench loss | No block; continue | HAZ-RING-001 |
| FM-004 | Sriov BDF missing, fallback forbidden | Open error | No silent Inject | Config/open tests | Fail fast | HAZ-IO-001 |
| FM-005 | Incomplete config | Parse error | No launch | demo-config tests | Refuse start | HAZ-CFG-001 |
| FM-006 | Inflated performance narrative | Misleading STR | Bad airborne decision | Report field checklist | `target_met` + backend labelled | HAZ-RATE-001 |
| FM-007 | Unlisted unsafe growth | TCB drift | Assurance gap | TCB inspection | Block release of docs gate | HAZ-TCB-001 |
| FM-008 | Claiming Sriov E2E before L2 ready | False Pass | Bad transfer decision | HARNESS gap table | Fail-fast smoke/bench | HAZ-IO-001, HAZ-RATE-001 |

*This table is not a complete FMEA; it orients design reviews for safety/airborne transfer.*

---

## 12. Assurance design (proofs and coverage)

| Layer | Mechanism | Gate |
|-------|-----------|------|
| Unit | Host tests on real ring / isolation / Inject / guest-common / hv EPT | STD unit procedures |
| Integration | Nested QEMU smoke (Inject/TCG as-built) | TEST-SMOKE-001 |
| Proof | Verus on `ring` + `isolation` (`docs/VERUS.md`) | TEST-VERUS-001 |
| Coverage | `cargo xtask coverage`, threshold 90%, TCB allowlist only | TEST-COV-001 |
| Measurement | Bench JSON (Inject harness sample) | TEST-BENCH-001 |

Declared coverage packages: `ring`, `isolation`, `pci-vf`, `guest-common`, `demo-config` (`xtask/coverage-allowlist.toml`).

---

## 13. Traceability SSS → design

| SYS / SG ID | Design element |
|-------------|----------------|
| SYS-ISO-001..003 / SG-ISO-* | `hv` EPT + exit path |
| SYS-ISO-004 | `isolation` crate |
| SYS-DP-001 / SG-ISO-002 | guests + `guest-common` + `ring` |
| SYS-DP-002..004 / SG-CFG-001 | `pci-vf` + config / resolve policy |
| SYS-DP-005..007 / SG-RATE-001 | stats + `xtask bench` + report metadata |
| SYS-AS-001..002 | Verus (`ring`/`isolation`) + coverage |
| SYS-AS-003..004 | `demo-config` + `xtask` |
| SYS-AS-005..007 | `docs/TCB.md` + `docs/system/` + independent V&V |
| HAZ-* | §11 FMEA-lite |

Full SWR → crate → TEST → STR matrix: HV-EDGE-STR-001 §9 / §9.1.

---

## 14. As-built status and open design points

### 14.1 Closed relative to Issue 0.3 ODPs

| ID | Topic | Resolution |
|----|-------|------------|
| ODP-001 | `IF-BOOT-*` layout | Implemented in `guest-common` / `hv::boot_blob`; ICD normative |
| ODP-002 | Dual-direction rings | Two logical directions via shared SPSC usage in forward path |
| ODP-003 | Verus stubs → hard-fail | Task 8: `ring` 4 + `isolation` 7 verified under hard-fail regime |
| ODP-004 | EPT-violation stimulus | Host unit tests for classify/halt policy; **nested** stimulus still open (see ODP-007) |

### 14.2 Remaining as-built gaps (honest)

| ID | Topic | Status | Blocks |
|----|-------|--------|--------|
| ODP-005 | WCET / bounded latency | Out of spike (NGX-008) | Airborne scheduling claims |
| ODP-006 | Integrity level (DAL/SIL) | Explicitly not assigned | Certification |
| ODP-007 | Nested EPT-violation demo | Blocked — no L2 VMLAUNCH | Full TEST-ISO-002 nested |
| ODP-008 | L2 VMLAUNCH + guest live poll | Incomplete (HARNESS gaps 1–2) | Sriov/Inject L2 smoke |
| ODP-009 | Sriov BAR/MMIO + BootInfo tips | Stub / zero (HARNESS gaps 3–4) | Real VF datapath |
| ODP-010 | Multiboot+KVM reliable entry | Flaky; TCG default (HARNESS gap 5) | Nested VT‑x under KVM |
| ODP-011 | Lab IOMMU/ACS VF isolation | Lab-specific (HARNESS gap 6) | QEMU vfio-pci open |

Source of truth for Sriov E2E gaps: `docs/HARNESS.md` Task 10 section.

---

## 15. Change record

| Issue | Date | Description |
|-------|------|-------------|
| 0.1 | 2026-10-26 | Skeleton |
| 0.2 | 2026-10-26 | Full SSDD: allocation, modes, TCB, security, assurance, ODPs |
| 0.3 | 2026-10-26 | Safety partitioning view; FMEA-lite; SG/HAZ traceability; airborne ODPs |
| 0.4 | 2026-10-26 | Task 11 as-built sync: guest-common, Multiboot/L1 smoke, Verus/coverage gates, HARNESS gaps, closed ODPs |
