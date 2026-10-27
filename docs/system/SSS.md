# System / Subsystem Specification (SSS)

| Field | Value |
|-------|-------|
| **Document ID** | HV-EDGE-SSS-001 |
| **Title** | Hypervisor Edge Gateway — System / Subsystem Specification |
| **Issue** | 0.3 |
| **Date** | 2026-10-26 |
| **Classification** | UNCLASSIFIED — Technology Demonstrator |
| **Parent suite** | HV-EDGE-DOC-SUITE |
| **Supersedes** | Issue 0.2 |
| **Transfer context** | Safety-related; airborne / aeronautical rates & partitioning (not certified) |

---

## Table of contents

1. [Document control](#1-document-control)  
2. [Purpose and applicability](#2-purpose-and-applicability)  
3. [Referenced documents](#3-referenced-documents)  
4. [Definitions and acronyms](#4-definitions-and-acronyms)  
5. [System overview and operational concept](#5-system-overview-and-operational-concept)  
6. [Stakeholders and external interfaces](#6-stakeholders-and-external-interfaces)  
7. [Safety goals and preliminary hazards](#7-safety-goals-and-preliminary-hazards)  
8. [System requirements (`SYS-*`)](#8-system-requirements-sys-)  
9. [Constraints, assumptions, and dependencies](#9-constraints-assumptions-and-dependencies)  
10. [Non-goals and exclusions](#10-non-goals-and-exclusions)  
11. [System success criteria (gates)](#11-system-success-criteria-gates)  
12. [Risks](#12-risks)  
13. [Requirements allocation summary](#13-requirements-allocation-summary)  
14. [Change record](#14-change-record)

---

## 1. Document control

### 1.1 Authority and status

This SSS defines **system-level needs and constraints** for the technology demonstrator. Software detail is deferred to HV-EDGE-SRS-001. Interface bit/byte contracts are deferred to HV-EDGE-ICD-001. Design realisation is described in HV-EDGE-SSDD-001.

### 1.2 Requirement language

| Verb | Meaning |
|------|---------|
| **shall** | Mandatory requirement for the demonstrator within stated scope |
| **should** | Programme objective; reported even if unmet |
| **may** | Permitted option |
| **will** | Statement of fact / external actor behaviour (not a design requirement) |

Verification methods used in this SSS: **T** Test, **D** Demonstration, **A** Analysis, **I** Inspection.

---

## 2. Purpose and applicability

### 2.1 Purpose

Specify the system needs for a **Type‑1 hypervisor–based edge gateway demonstrator** that:

- isolates WAN-facing and LAN-facing processing domains in hardware-assisted virtual machines;  
- forwards traffic host → WAN guest → shared ring → LAN guest → host (and reverse);  
- measures bidirectional throughput at **64-byte** frames with a published report;  
- subjects provable software to **machine-checked specifications** (Verus) and enforces **≥ 90%** structural coverage on the declared software perimeter.

### 2.2 Applicability

| Applies to | Does not apply to |
|------------|-------------------|
| Nested execution under QEMU/KVM with Intel VT‑x (`+vmx`) | Production bare-metal certification |
| Software artefacts in repository `jblestang/AI-Pervisor` on the spike baseline | Third-party OS guests (Linux, etc.) |
| Laboratory configurations described by `config/demo.toml` + ICD | Sites without nested virtualisation capability (blocked, not waived by mock) |
| **Transfer studies** toward safety-related or airborne edge gateways (documentary + measured rates) | Claiming a DAL/SIL/ASIL or airworthiness approval for this baseline |

### 2.2.1 Safety-related and airborne transfer note

Programme intent includes possible later use in **safety-related** contexts (functions whose incorrect behaviour could contribute to harm or loss of mission-critical traffic segregation) and in **airborne / aeronautical** contexts where **on-air / airborne packet rates**, isolation between network domains, and predictable failure modes are scrutinised.

This SSS therefore:

- states **preliminary safety goals** and **hazards** (§7) to orient design and V&V;  
- treats **measured bidirectional rates @ 64 B** as evidence artefacts for rate discussions with airborne stakeholders;  
- **does not** assign or claim an assurance/integrity level for the demonstrator software.

### 2.3 System of interest boundary

**Inside:** L1 hypervisor (`hv`), WAN guest, LAN guest, shared ring and isolation model, `pci-vf` backends, operator harness (`xtask`), configuration schema, documentary suite.

**Outside:** L0 host OS/KVM, physical NIC silicon beyond VF assignment, enterprise orchestration, cryptographic key management, network policy beyond frame forward/measure.

---

## 3. Referenced documents

| Ref | Document | Use |
|-----|----------|-----|
| R1 | Design: Hyperviseur edge gateway Rust — spike (2026-10-26) | Source of programme decisions |
| R2 | Implementation plan: Hyperviseur edge spike (2026-10-26) | Work breakdown |
| R3 | HV-EDGE-SSDD-001 | Design description |
| R4 | HV-EDGE-SRS-001 | Software requirements |
| R5 | HV-EDGE-ICD-001 | Interfaces |
| R6 | HV-EDGE-STD-001 / HV-EDGE-STR-001 | V&V description / report |
| R7 | Intel® 64 and IA-32 Architectures SDM (VT‑x / EPT) | Hardware behaviour reference |
| R8 | `docs/VERUS.md`, `docs/TCB.md`, `docs/HARNESS.md` | Toolchain, TCB, lab procedures |

---

## 4. Definitions and acronyms

| Term | Definition |
|------|------------|
| **Type‑1 hypervisor** | Hypervisor that owns CPU virtualisation extensions and memory virtualisation (EPT) in its own code path, without a general-purpose host OS inside the TCB of the guests |
| **WAN / LAN guest** | Isolated `no_std` Rust binaries representing edge network domains |
| **Inject backend** | Production-degraded host↔guest I/O path using an in-process frame queue when SR‑IOV VF is unavailable **and** configuration permits |
| **Mock** | Test double that substitutes for a component under test — **forbidden** for ring, isolation model, and Inject/Sriov behavioural substitutes used to “green” tests |
| **TCB** | Trusted Computing Base — residual `unsafe` / unproved hardware-facing code listed in `docs/TCB.md` |
| **EPT** | Extended Page Tables (Intel) |
| **SR‑IOV / VF** | Single Root I/O Virtualisation / Virtual Function |
| **SPSC** | Single-producer / single-consumer |
| **V&V** | Verification and Validation |

| Acronym | Expansion |
|---------|-----------|
| BDF | Bus:Device.Function (PCI) |
| CONOPS | Concept of Operations |
| GPA | Guest Physical Address |
| IOMMU | Input–Output Memory Management Unit |
| VFIO | Virtual Function I/O |
| TRL | Technology Readiness Level |
| DAL | Design Assurance Level (airborne software — referenced informatively only) |
| SIL | Safety Integrity Level (industrial — referenced informatively only) |
| PHA | Preliminary Hazard Analysis (lightweight, demonstrator-scoped) |
| FCC | Failure containment / fail-closed behaviour (suite term) |

---

## 5. System overview and operational concept

### 5.1 Mission context

Edge gateways must separate untrusted or differently trusted network domains while sustaining high packet rates. In safety-related or airborne transfer, that separation is a **partitioning barrier**; rates @ small frames are a **performance and timing stress** on that barrier. This demonstrator explores a **minimal Type‑1** approach in Rust with:

- hardware-assisted isolation (VT‑x / EPT);  
- a zero-copy shared-memory datapath between domains;  
- measurable performance under worst-case small frames;  
- formal specification of safe software and an explicit TCB.

### 5.2 Concept of operations (CONOPS)

**Actors**

| Actor | Role |
|-------|------|
| Laboratory operator | Configures `demo.toml`, binds VFs if available, runs `cargo xtask` commands |
| L0 host (QEMU/KVM) | Provides nested VT‑x, optional IOMMU/VFIO passthrough |
| L1 `hv` | Creates two guests, builds EPT maps, emulates UART, loads boot info |
| WAN guest | Host-facing NIC + forward to LAN via ring |
| LAN guest | Host-facing NIC + forward to WAN via ring |
| Independent V&V | Executes STD procedures, archives evidence, completes STR |

**Nominal operational thread (smoke)**

1. Operator supplies complete configuration (or system refuses to start).  
2. Harness builds images and launches nested QEMU per config.  
3. `hv` enters VMX, maps private and shared regions, starts both guests.  
4. Frames traverse NIC → WAN → ring → LAN → NIC (and reverse as configured).  
5. Counters update; serial emits `SMOKE_OK` on success.  
6. Operator or automation archives logs as STD evidence.

**Measurement thread (bench)**

As smoke, then generate 64-byte bidirectional load for configured duration; publish JSON report with Gbps, pps, loss, and `target_met` against configured `target_gbps` (objective: 1.0 Gbps).

### 5.3 Context diagram

```text
                 ┌──────────────────────────────────────────────────┐
   Physical or   │              Laboratory host (L0)                │
   synthetic     │   QEMU nested (+vmx) · IOMMU · VFIO (optional)   │
   traffic       │                                                  │
        │        │    ┌─────────────── System of interest ────────┐ │
        │        │    │  hv (L1)  ·  EPT  ·  UART emu  ·  boot    │ │
        └────────┼───►│     ┌─────────┐  ring   ┌─────────┐       │ │
                 │    │     │ WAN VM  │◄───────►│ LAN VM  │       │ │
                 │    │     │ NicB/E  │ shared  │ NicB/E  │       │ │
                 │    │     └─────────┘  mem    └─────────┘       │ │
                 │    └───────────────────────────────────────────┘ │
                 │              ▲  xtask / serial / reports         │
                 │              │                                   │
                 │         Operator / V&V                           │
                 └──────────────────────────────────────────────────┘
```

---

## 6. Stakeholders and external interfaces

| Stakeholder | Interest |
|-------------|----------|
| Programme owner | Demonstrator gates, TRL evidence, schedule |
| Systems engineering | Completeness of needs, allocation, risks |
| Software engineering | Implementable shalls, ICD stability |
| Hardware / platform engineering | Nested VT‑x, IOMMU, VF availability |
| Independent test | Repeatable procedures, objective pass/fail |
| Governmental / institutional reviewer | Traceability, honesty of scope, evidence quality |
| Safety engineering (transfer) | Hazards, partitioning, fail behaviour, independence of V&V |
| Airborne / aeronautical systems (transfer) | Domain isolation, measured rates @ 64 B, TCB clarity, no false DAL claim |

**External interfaces (system level)**

| Interface | Counterpart | Control doc |
|-----------|-------------|-------------|
| Nested virtualisation / QEMU CLI | L0 lab | ICD `IF-CFG-LAB`, HARNESS |
| PCI VF (SR‑IOV) | NIC / VFIO | ICD `IF-NIC-*`, HARNESS |
| Operator configuration | File + environment | ICD `IF-CFG-*` |
| Serial / UART observation | Operator / smoke tool | ICD `IF-UART-*`, `IF-SER-*` |
| Bench report file | V&V archive | STD / STR |

---

## 7. Safety goals and preliminary hazards

> **Status:** Preliminary, demonstrator-scoped. Not a complete PHA/PSSA/SSA. Intended to make safety/airborne reviewers see that isolation and rate behaviour are treated as first-class concerns.

### 7.1 Safety goals (`SG-*`)

| ID | Safety goal | Related SYS |
|----|-------------|-------------|
| SG-ISO-001 | Prevent a fault or malicious behaviour in one guest domain (WAN or LAN) from **silently corrupting** the private memory or control state of the other domain. | SYS-ISO-001..003 |
| SG-ISO-002 | Ensure that the only intentional cross-domain data path is the **ICD-defined** shared ring / arena / stats surface — not an accidental mapping. | SYS-ISO-002, SYS-DP-001 |
| SG-FCC-001 | On isolation violation or unrecoverable guest fault, **fail loud** (diagnostic + halt faulting domain) rather than continue with undefined cross-domain state. | SYS-ISO-003 |
| SG-CFG-001 | Prevent demonstrator launch with incomplete or inconsistent I/O backend configuration that would silently weaken the intended host path (e.g. undeclared substitute for Sriov). | SYS-DP-003..004, SYS-AS-003 |
| SG-RATE-001 | Provide **published, reproducible** bidirectional rate evidence at worst-case 64-byte frames so airborne/safety stakeholders can judge suitability — without inventing unmet figures. | SYS-DP-005..006 |

### 7.2 Preliminary hazards (`HAZ-*`)

| ID | Hazard (undesired condition) | Possible cause (examples) | Severity class (demonstrator) | Mitigations in this design |
|----|------------------------------|---------------------------|-------------------------------|----------------------------|
| HAZ-MEM-001 | Cross-domain memory corruption | EPT misconfiguration; shared page over-mapped | High (for transfer) | EPT private maps; `isolation` overlap reject; violation halt |
| HAZ-IO-001 | Undeclared weakening of host I/O path | Silent mock / fallback without config | Medium–High | Fail-fast Sriov; Inject only if config allows; same API |
| HAZ-RING-001 | Indefinite block or livelock on full ring | Blocking producer under load | Medium | Drop + counter (SWR-DP-013); bench/smoke observe drops |
| HAZ-CFG-001 | Wrong lab profile embedded in binary | Hardcoded BDF/sizes | Medium | Config-only parameters; reject incomplete TOML |
| HAZ-RATE-001 | Unsubstantiated performance claim for airborne use | Missing or synthetic measurement | Medium | Mandatory bench report; `target_met` honesty |
| HAZ-TCB-001 | Unbounded unsafe surface | Unlisted `unsafe` / unproved HW glue | High (for transfer) | TCB enumeration; Verus on safe perimeter |

### 7.3 Informative assurance-level mapping (non-claiming)

| If a follow-on programme targets… | This demonstrator contributes… | This demonstrator does **not** provide… |
|-----------------------------------|--------------------------------|----------------------------------------|
| Airborne software (e.g. DAL framing) | Partitioning concept, requirements IDs, ICD, V&V procedures, rate reports, TCB list | Process assurance, tool qualification, SOI, certified compiler story |
| Industrial safety (e.g. SIL framing) | Fail-closed config, isolation tests, coverage/proof gates | Full IEC 61508 lifecycle, certified PE |
| Automotive ASIL transfer | Domain isolation pattern, evidence hooks | ISO 26262 work products |

---

## 8. System requirements (`SYS-*`)

### 8.1 Isolation and safety of domains

| ID | Requirement | Priority | V&V |
|----|-------------|----------|-----|
| SYS-ISO-001 | The system **shall** isolate WAN guest private memory from LAN guest private memory using EPT such that a guest cannot read or write the other guest’s private region. | Must | T/D |
| SYS-ISO-002 | The system **shall** map shared ring (and other explicitly designated shared pages) read-write to both guests according to the ICD memory map. | Must | T/I |
| SYS-ISO-003 | On EPT violation for an unauthorised access, the system **shall** handle the exit without silent corruption of the peer guest’s memory, emitting a diagnostic on the UART path and halting the faulting guest. | Must | T/D |
| SYS-ISO-004 | The system **shall** maintain a software region/permission model used to construct EPT intent; conflicting private WAN/LAN claims **shall** be rejected by that model. | Must | T/A |

### 8.2 Datapath and performance (including airborne rate evidence)

| ID | Requirement | Priority | V&V |
|----|-------------|----------|-----|
| SYS-DP-001 | The system **shall** provide bidirectional frame forwarding host↔WAN↔shared-ring↔LAN↔host under nested execution. | Must | T/D |
| SYS-DP-002 | Host↔guest I/O **shall** use SR‑IOV Virtual Functions when configuration selects `sriov` and devices are present. | Must | D |
| SYS-DP-003 | The system **may** use the Inject backend only when configuration selects `inject`, or when `allow_inject_fallback` is true and no VF is available; Inject **shall not** be used as an undeclared test mock. | Must | T/I |
| SYS-DP-004 | When configuration requires `sriov` and the VF is absent, the system **shall** fail fast (refuse silent substitution). | Must | T |
| SYS-DP-005 | The system **shall** measure and publish E2E Gbps, pps, and loss for bidirectional traffic at 64-byte frames for a configured duration. | Must | T |
| SYS-DP-006 | The system **should** achieve ≥ 1.0 Gbps bidirectional at 64-byte frames under laboratory SR‑IOV conditions; unmet targets **shall** still produce a valid report with `target_met = false`. | Should | T |
| SYS-DP-007 | Published rate reports **shall** identify the I/O backend, frame size, duration, loss, and environment class so airborne/safety reviewers can interpret **on-air / lab rates** without ambiguity. | Must | I/T |

### 8.3 Assurance, configuration, and operability

| ID | Requirement | Priority | V&V |
|----|-------------|----------|-----|
| SYS-AS-001 | Provable safe software in the declared perimeter **shall** be subject to Verus specification checking via the operator harness; harness verify **shall** fail when proofs are broken under the Task‑8 hard-fail regime. | Must | T |
| SYS-AS-002 | Structural line coverage on the declared crate set **shall** be ≥ 90%, enforced by the coverage harness command. | Must | T |
| SYS-AS-003 | All demonstrator parameters (VF BDF, memory sizes, ring capacity, bench settings, image paths, QEMU parameters) **shall** be supplied by configuration and/or documented environment overrides; incomplete configuration **shall** cause start failure. | Must | T/I |
| SYS-AS-004 | The system **shall** provide operator commands: build, run-nested, verify, smoke, bench, coverage, and gdb attach support, invoked through Cargo `xtask` only (no parallel Make/just requirement). | Must | D/I |
| SYS-AS-005 | Residual `unsafe` / unproved hardware-facing code **shall** be enumerated in a TCB document. | Must | I |
| SYS-AS-006 | The documentary suite (SSS, SSDD, SRS, ICD, STD, STR) **shall** be maintained in English under `docs/system/` with stable requirement identifiers. | Must | I |
| SYS-AS-007 | For campaigns intended for governmental or safety/airborne review, verification of Must isolation and rate gates **should** be conducted or countersigned by personnel organisationally independent of the implementing developers. | Should | I |

---

## 9. Constraints, assumptions, and dependencies

### 9.1 Constraints (`CON-*`)

| ID | Constraint |
|----|------------|
| CON-PLT-001 | Initial target architecture is **x86_64 Intel** with nested VT‑x. |
| CON-SW-001 | Hypervisor and guests are implemented in **Rust** `no_std` + `alloc` (except std tools: `xtask`, `demo-config`). |
| CON-SW-002 | Build and operator entry points are **Cargo workspace + `cargo xtask`**. |
| CON-QA-001 | No business mocks for ring, isolation, or NIC backends in unit tests; Inject is a real backend, not a Sriov mock. |
| CON-DOC-001 | Documentary structure is home-grown; IEEE/ECSS/DO‑178 templates are **not** mandated and **not** claimed. |
| CON-SAF-001 | No DAL/SIL/ASIL is asserted for this baseline; safety goals in §7 are preliminary. |

### 9.2 Assumptions (`ASM-*`)

| ID | Assumption | If violated |
|----|------------|-------------|
| ASM-LAB-001 | Laboratory provides nested VT‑x (`+vmx`) under KVM. | Nested smoke/bench blocked |
| ASM-LAB-002 | For official Sriov results, laboratory provides IOMMU and bindable VFs. | Official bench may run Inject only if config allows; mark STR accordingly |
| ASM-TOOL-001 | Rust toolchain matches Verus requirements (see `docs/VERUS.md`; currently 1.99.1). | Verify soft-skip or fail per VERUS.md policy |
| ASM-OP-001 | Operator can supply a complete `demo.toml`. | System refuses boot (by design) |

### 9.3 Dependencies

| Dependency | Nature |
|------------|--------|
| QEMU + KVM | Nested execution substrate |
| Verus release aligned to rustc pin | Proof checking |
| `cargo-llvm-cov` (or equivalent via xtask) | Coverage gate |
| Optional: physical NIC with SR‑IOV | Production-representative datapath / airborne-like rate path |

---

## 10. Non-goals and exclusions

| ID | Exclusion | Rationale |
|----|-----------|-----------|
| NGX-001 | Bare-metal Intel boot | Deferred post-demonstrator |
| NGX-002 | Linux or other GPOS guests | Scope control |
| NGX-003 | Machine-checked proofs of hardware EPT walks beyond the Rust isolation model | TCB boundary |
| NGX-004 | ARM / RISC‑V | Deferred |
| NGX-005 | Cryptographic isolation / MLS labelling | Out of demonstrator scope |
| NGX-006 | Formal DO‑178C / ED‑12, ARP4754A/4761, ECSS, IEC 61508, or ISO 26262 **process** qualification or **product** certification | Not claimed — architecture is transfer-oriented only |
| NGX-007 | Assignment of DAL / SIL / ASIL to this baseline | Explicitly deferred to a follow-on safety programme |
| NGX-008 | Deterministic worst-case execution time (WCET) proof for airborne scheduling | Out of spike; rates are measured, not WCET-certified |

---

## 11. System success criteria (gates)

The demonstrator baseline is **accepted for technical review closure** when all **Must** items below are evidenced in STR (Should items reported):

| Gate | Evidence |
|------|----------|
| G1 Smoke | `TEST-SMOKE-001` Pass — two guests, ring path, `SMOKE_OK` |
| G2 Verify | `TEST-VERUS-001` Pass under applicable hard-fail policy |
| G3 Isolation | `TEST-ISO-001` / `TEST-ISO-002` Pass — EPT intent + violation handling |
| G4 Bench report | `TEST-BENCH-001` Pass — valid JSON report @ 64 B (target optional) |
| G5 Coverage | `TEST-COV-001` Pass — ≥ 90% |
| G6 Config integrity | `TEST-CFG-001` Pass + inspection — no demo hardcoding |
| G7 Docs | `TEST-DOC-001` Pass — suite complete; STR filled |
| G8 Safety/airborne honesty | STR states non-certification; rate report fields complete (SYS-DP-007); SG/HAZ reviewed |

---

## 12. Risks

| ID | Risk | Impact | Mitigation | Residual |
|----|------|--------|------------|----------|
| RSK-001 | Nested + real SR‑IOV difficult in lab | Official Gbps figures delayed | Dual backend API; early HARNESS; STR Blocked vs Pass | Medium |
| RSK-002 | 1 Gbps @ 64 B (~1.5 Mpps/dir) hard under nested | Should-gate unmet; airborne rate discussions constrained | Mandatory measurement; optimise ring/poll; do not fake Pass | Accepted for spike |
| RSK-003 | Verus × `no_std` friction | Proof lag | Stub→real contracts plan; VERUS.md; Task 8 hard-fail | Medium |
| RSK-004 | Type‑1 scope vs schedule | Incomplete vertical slice | xtask-first; vertical cut; honest STR Not run | Medium |
| RSK-005 | Reviewers misread demonstrator as certified | Programme / legal exposure | Explicit non-claim language in suite Issue 0.3+; STR §1 | Low if followed |
| RSK-006 | Shared ring becomes common-cause channel | Safety transfer weakness | Minimise shared surface (ICD); drop-on-full; document as intentional coupling in SSDD | Medium |

---

## 13. Requirements allocation summary

| SYS ID | Primary allocation | Detailed in |
|--------|--------------------|-------------|
| SYS-ISO-* | `hv` EPT + `isolation` | SSDD §4–5, SRS SWR-ISO-* |
| SYS-DP-001..004 | guests + `ring` + `pci-vf` | SSDD §5–6, SRS SWR-DP-*, SWR-NIC-* |
| SYS-DP-005..007 | `xtask` bench + stats + report metadata | SRS SWR-HVN-*, ICD IF-STATS-* |
| SYS-AS-001..002 | Verus + coverage harness | SRS SWR-AS-*, STD |
| SYS-AS-003..004 | `demo-config` + `xtask` | ICD IF-CFG-*, SRS SWR-CFG-* |
| SYS-AS-005..007 | docs / TCB / independent V&V | SSDD §8, SRS SWR-DOC-*, STD |
| SG-* / HAZ-* | Design & V&V orientation | SSDD partitioning / FMEA-lite |

---

## 14. Change record

| Issue | Date | Description |
|-------|------|-------------|
| 0.1 | 2026-10-26 | Skeleton |
| 0.2 | 2026-10-26 | Full SSS: CONOPS, shall language, stakeholders, gates, risks, allocation |
| 0.3 | 2026-10-26 | Safety goals / PHA-lite; airborne rate evidence; SYS-DP-007 / SYS-AS-007; non-cert exclusions |
