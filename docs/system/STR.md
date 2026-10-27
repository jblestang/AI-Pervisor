# Software Test Report (STR)

| Field | Value |
|-------|-------|
| **Document ID** | HV-EDGE-STR-001 |
| **Title** | Hypervisor Edge Gateway — Software Test Report |
| **Issue** | 0.4 (campaign filled) |
| **Date** | 2026-10-26 |
| **Classification** | UNCLASSIFIED — Technology Demonstrator |
| **Parent suite** | HV-EDGE-DOC-SUITE |
| **Procedures** | HV-EDGE-STD-001 Issue 0.3 |
| **Requirements** | HV-EDGE-SRS-001 Issue 0.3 |
| **Design** | HV-EDGE-SSDD-001 Issue 0.4 |
| **Status** | **Campaign complete** for spike baseline (Tasks 0–11) |
| **Certification claim** | **None** — not DAL/SIL/ASIL/airworthiness approved |

---

## Table of contents

1. [Purpose and limitations](#1-purpose-and-limitations)  
2. [Campaign identification](#2-campaign-identification)  
3. [Executive summary](#3-executive-summary)  
4. [Environment as tested](#4-environment-as-tested)  
5. [Results by procedure](#5-results-by-procedure)  
6. [Coverage results](#6-coverage-results)  
7. [Verus results](#7-verus-results)  
8. [Bench measurement results](#8-bench-measurement-results)  
9. [Requirement verification and traceability](#9-requirement-verification-and-traceability)  
10. [Anomalies, deviations, and waivers](#10-anomalies-deviations-and-waivers)  
11. [Conclusions and recommendations](#11-conclusions-and-recommendations)  
12. [Approvals](#12-approvals)  
13. [Change record](#13-change-record)

---

## 1. Purpose and limitations

This report records the **outcome of the STD verification campaign** against the spike software baseline. It is intended for programme management and external technical authority review.

**Limitations (standing):**

- Demonstrator under nested QEMU (TCG Multiboot / lab KVM capability) is not a bare-metal, safety, or airborne **certification** campaign.  
- No Design Assurance Level (DAL), Safety Integrity Level (SIL), ASIL, or airworthiness approval is claimed or implied by Pass results.  
- Smoke and bench Pass results for this campaign are on the **Inject** datapath (L1 inject harness / inject-harness). They are **not** full Sriov L2 E2E.  
- Should-priority official Sriov figures (SWR-HVN-055) remain unmet; Inject rates are labelled `environment_class=inject-harness`.  
- Nested EPT-violation demonstration (TEST-ISO-002) is **Blocked** pending L2 VMLAUNCH (host exit-policy unit tests Pass separately — see §5 notes).

---

## 2. Campaign identification

| Field | Value |
|-------|-------|
| Campaign ID | HV-EDGE-VV-SPIKE-2026-10-26 |
| Campaign name | Hypervisor edge spike — Tasks 7–11 closure |
| Execution window | 2026-10-26 (Tasks 7–10 evidence; Task 11 documentary close) |
| STD Issue | HV-EDGE-STD-001 Issue 0.3 |
| SRS Issue | HV-EDGE-SRS-001 Issue 0.3 |
| SSS Issue | HV-EDGE-SSS-001 Issue 0.3 (no scope drift) |
| SSDD Issue | HV-EDGE-SSDD-001 Issue 0.4 |
| Independent tester (Y/N) | N — developer/agent campaign (SYS-AS-007 independence recommended for transfer) |
| Safety/airborne review intent (Y/N) | Y — transfer framing; **no** certification claim |
| Git repository | `https://github.com/jblestang/AI-Pervisor` |
| Git branch | `cursor/spike-hypervisor-edge-7e29` |
| Git commit SHA (docs close) | Branch tip after Task 11 (`git log -1 --grep='documentary suite with STR'`) |
| Evidence pack location | Repository artefacts + prior task logs; sample bench JSON under `docs/system/samples/` |
| Operator | Spike implementation workers (Tasks 7–11) |
| Organisation | AI-Pervisor / Hyperviseur programme |

### 2.1 Evidence baseline SHAs

| Artefact | SHA | Notes |
|----------|-----|-------|
| Smoke Pass | `51781398fdb9476ed9b3090c645ef68f14b0458b` | Task 7 — `SMOKE_OK`, Inject, TCG |
| Coverage + Verus Pass | `01f8a69f1497bf78883e5a01964e76e87fe2a525` | Task 8 — 97.23%; ring 4 / isolation 7 |
| Bench Inject sample | `541e0b61ca4965ec0f65a07964b5524df95feb7b` | Task 9 — ~2.24 Gbps @ 64 B |
| Sriov lab path / HARNESS gaps | `a5b941e15e8c53f564e21cf6b869d6797c4ff30f` | Task 10 |
| Documentary suite close | `22bc6055771f63ee01bd04191624b0be22dda8db` | Task 11 |

---

## 3. Executive summary

| Metric | Value |
|--------|-------|
| Procedures planned | 13 |
| **Pass** | **11** |
| **Fail** | **0** |
| **Blocked** | **2** |
| Not run | 0 |
| Must-gate status (smoke, coverage, verify, config, units, docs) | **Pass** on Inject/HOST where applicable |
| Should-gate (1 Gbps @ 64 B official Sriov) | **Not met as Sriov** — Inject sample `target_met=true` (~2.24 Gbps); Sriov E2E Blocked |
| Recommendation | Spike documentary suite **closed** for Tasks 0–11; do **not** claim Sriov L2 E2E or any DAL/SIL/airworthiness |

**Narrative:** Host/component gates, Inject smoke, Inject bench measurement, coverage ≥90%, and Verus on annotated crates all Pass. Full nested Sriov datapath and nested EPT-violation stimulus remain Blocked per HARNESS Task 10 gaps. **Follow-on (post Task 11):** L2 VMCS/`VMLAUNCH` path, guest live poll, and BootInfo `nic_mmio_*` wiring landed in software; Multiboot+KVM runtime entry and VF descriptor rings remain the open blockers for nested UART / Sriov E2E evidence (see HARNESS gap table).

---

## 4. Environment as tested

| Item | Value |
|------|-------|
| Environment class | **HOST** + **LAB-INJECT** (TCG Multiboot); LAB-SRIOV path documented but E2E Blocked |
| Host CPU / model | Lab agent host with `/dev/kvm` and `kvm_intel nested=1` capability signals |
| Nested VT‑x confirmed | Capability present; Multiboot **KVM entry flaky** — smoke default **TCG** (`HV_QEMU_ACCEL=tcg`) |
| QEMU version | `qemu-system-x86_64` per `[lab].qemu_bin` |
| Kernel / KVM | Nested parameter readable; smoke did not require successful L2 VMLAUNCH |
| IOMMU | Not required for Inject campaign; Sriov checklist in HARNESS |
| WAN VF BDF | N/A for Inject Pass; Sriov requires vfio-bound BDFs |
| LAN VF BDF | N/A for Inject Pass |
| `datapath.backend` | `inject` (smoke/bench Pass); `sriov` resolve-tested / fail-fast when L2 incomplete |
| `allow_inject_fallback` | `true` for Inject configs; `false` for official Sriov (fail-fast) |
| Config file | `config/demo.example.toml` / local demo; bench: `config/bench.inject.ci.toml` |
| Rust toolchain | **1.99.1** (`rust-toolchain.toml`) |
| Verus version | `0.2026.10.26.d17fbd0` / `vstd=0.0.0-2026-10-25-0158` |

---

## 5. Results by procedure

| TEST ID | Result | Date | Evidence reference | Notes |
|---------|--------|------|--------------------|-------|
| TEST-BUILD-001 | **Pass** | 2026-10-26 | Task 7 log; `cargo xtask build` | Freestanding `hv` / guests + `hv.bin` objcopy |
| TEST-CFG-001 | **Pass** | 2026-10-26 | `cargo test -p demo-config` | Incomplete reject + example load + env overrides |
| TEST-UNIT-RING | **Pass** | 2026-10-26 | `cargo test -p ring` | Real SPSC host tests; no ring mock |
| TEST-UNIT-ISO | **Pass** | 2026-10-26 | `cargo test -p isolation` | Region allow/deny + overlap reject |
| TEST-UNIT-NIC | **Pass** | 2026-10-26 | `cargo test -p pci-vf` | Inject behaviour + Sriov fail-fast |
| TEST-UNIT-FWD | **Pass** | 2026-10-26 | `cargo test -p guest-common` | Real ring + Inject in-process forward |
| TEST-VERUS-001 | **Pass** | 2026-10-26 | Task 8; `cargo xtask verify` | Hard-fail regime: ring **4**, isolation **7** verified |
| TEST-COV-001 | **Pass** | 2026-10-26 | Task 8; `cargo xtask coverage` | **97.23%** lines (666/685); threshold 90% |
| TEST-ISO-001 | **Pass** | 2026-10-26 | `cargo test -p hv --features std` | EPT intent / map / fault classify host tests |
| TEST-ISO-002 | **Blocked** | 2026-10-27 | HARNESS gap 5 (Multiboot+KVM) | L2 enter code + host VMCS/PT tests Pass; nested stimulus still needs working KVM Multiboot (TCG has no VMXON). Host exit-policy unit tests Pass separately |
| TEST-SMOKE-001 | **Pass** | 2026-10-26 | Task 7; serial `SMOKE_OK` | Inject + TCG Multiboot. Sriov-resolved E2E **Blocked** (fail-fast; no silent Inject) |
| TEST-BENCH-001 | **Pass** | 2026-10-26 | Task 9; sample JSON | Inject harness @ 64 B; report complete; `target_met=true`. Sriov E2E **Blocked** |
| TEST-DOC-001 | **Pass** | 2026-10-26 | Task 11; `docs/system/` + TCB/HARNESS/VERUS | Suite Issue ≥0.3; non-cert statement; STR filled; SSDD as-built |

Result vocabulary: **Pass** | **Fail** | **Blocked** | **Not run** (per STD §8).

**Procedure counts:** Pass **11** / Fail **0** / Blocked **2** / Not run **0**.

---

## 6. Coverage results

| Item | Value |
|------|-------|
| Command | `cargo xtask coverage` |
| Report path | `coverage/llvm-cov-summary.json` (generated at run) |
| Declared crates | `ring`, `isolation`, `pci-vf`, `guest-common`, `demo-config` |
| Line coverage % | **97.23%** (97.2263%; 666/685 lines) |
| Threshold | 90% |
| Threshold met | **Yes** |
| Allowlist path | `xtask/coverage-allowlist.toml` |
| Allowlist review | TCB-only exclusions: bump_alloc, guest uart, sriov.rs, verus_stub — each with justification |

---

## 7. Verus results

| Item | Value |
|------|-------|
| Command | `cargo xtask verify` |
| Log path | Console / Task 8 campaign capture |
| Crates checked | `ring`, `isolation` (`[package.metadata.verus] verify = true`) |
| Regime | **Hard-fail** when Verus present; soft-skip only if toolchain missing (documented) |
| Outcome | **Pass** — `ring` **4** verified, `isolation` **7** verified |
| Toolchain | Rust **1.99.1** + Verus `0.2026.10.26.d17fbd0` |
| Non-conformances | `pci-vf` / `guest-common` retain stub intent only (`verify = false`) — recorded in `docs/VERUS.md`, not a campaign Fail |

---

## 8. Bench measurement results

| Field | Value |
|-------|-------|
| Report path | `target/bench-report.json` (runtime); sample `docs/system/samples/bench-report.example.json` |
| Config | `config/bench.inject.ci.toml` |
| Frame size (B) | **64** |
| Duration (s) | **1** |
| Bidirectional | **true** |
| Backend used | **inject** |
| Environment class | **inject-harness** |
| Gbps | **~2.24** (sample 2.241386 aggregate bidirectional TX bytes) |
| pps | **~4.38e6** (sample 4377707.538) |
| Loss | **0.0** |
| `target_gbps` | **1.0** |
| `target_met` | **true** |
| IF-RPT-BENCH complete | **Yes** (backend, environment_class, target_met, etc.) |
| Procedure result | **Pass** (measurement completed; target met on Inject) |
| Sriov L2 E2E | **Not run / Blocked** — HARNESS Task 10 gaps; tool fails fast rather than publishing Inject as Sriov |
| Airborne/safety interpretation note | Rates are **nested-lab Inject harness**, not ENV-LAB-SRIOV + Sriov. Do not use as official Sriov or flight-clearance figures. |

---

## 9. Requirement verification and traceability

### 9.1 Traceability table — SRS ID → crate → STD → STR result

| SWR ID | Priority | Primary crate(s) | TEST ID(s) | STR result | Remarks |
|--------|----------|------------------|------------|------------|---------|
| SWR-ISO-001 | Must | `hv` | TEST-ISO-001, TEST-SMOKE-001 | **Pass** | EPT private maps host-tested; smoke Inject |
| SWR-ISO-002 | Must | `hv`, `guest-common` | TEST-ISO-001 | **Pass** | Shared ring/arena/stats mapping intent |
| SWR-ISO-003 | Must | `hv` | TEST-ISO-002 | **Blocked** | Nested stimulus blocked; host halt policy tested |
| SWR-ISO-004 | Must | `isolation` | TEST-UNIT-ISO, TEST-VERUS-001 | **Pass** | Overlap reject + Verus |
| SWR-ISO-005 | Must | `isolation` | TEST-UNIT-ISO | **Pass** | R/W/X deny |
| SWR-DP-010 | Must | `guest-wan`, `guest-lan` | TEST-BUILD-001 | **Pass** | no_std images |
| SWR-DP-011 | Must | `guest-common` | TEST-SMOKE-001, TEST-UNIT-FWD | **Pass** | Bidirectional forward |
| SWR-DP-012 | Must | `guest-common`, `hv` | TEST-CFG-001, TEST-DOC-001 | **Pass** | BootInfo / no lab literals |
| SWR-DP-013 | Must | `ring`, `guest-common` | TEST-UNIT-RING | **Pass** | Full → drop + counter |
| SWR-DP-014 | Must | `guest-common` | TEST-SMOKE-001, TEST-BENCH-001 | **Pass** | Stats updates |
| SWR-DP-015 | Must | `ring` | TEST-UNIT-RING, TEST-VERUS-001 | **Pass** | SPSC + proofs |
| SWR-NIC-020 | Must | `pci-vf` | TEST-UNIT-NIC | **Pass** | NicBackend API |
| SWR-NIC-021 | Must | `pci-vf`, `xtask` | TEST-SMOKE-001 | **Blocked*** | *Sriov open/resolve Pass; L2 Sriov smoke Blocked — see ANO-001 |
| SWR-NIC-022 | Must | `pci-vf` | TEST-UNIT-NIC, TEST-SMOKE-001 | **Pass** | Inject selection |
| SWR-NIC-023 | Must | `pci-vf`, `xtask` | TEST-UNIT-NIC | **Pass** | Fail-fast; no silent substitute |
| SWR-NIC-024 | Must | `guest-*`, `pci-vf` | TEST-DOC-001, TEST-SMOKE-001 | **Pass** | Config-only switch |
| SWR-CFG-030 | Must | `demo-config` | TEST-CFG-001 | **Pass** | TOML sections |
| SWR-CFG-031 | Must | `demo-config` | TEST-CFG-001 | **Pass** | Incomplete reject |
| SWR-CFG-032 | Must | `demo-config` | TEST-CFG-001 | **Pass** | Env overrides |
| SWR-CFG-033 | Must | `hv`, guests | TEST-DOC-001 | **Pass** | No demo hardcoding |
| SWR-CFG-034 | Must | repo `config/` | TEST-DOC-001 | **Pass** | `demo.example.toml` |
| SWR-CFG-035 | Must | `xtask` | TEST-SMOKE-001 | **Pass** | QEMU argv from DemoConfig |
| SWR-AS-040 | Must | `ring`, `isolation` | TEST-VERUS-001 | **Pass** | Hard-fail proofs |
| SWR-AS-041 | Must | `docs/TCB.md` | TEST-DOC-001 | **Pass** | TCB listed |
| SWR-AS-042 | Must | `xtask` | TEST-COV-001 | **Pass** | ≥90% enforced |
| SWR-AS-043 | Must | `xtask` allowlist | TEST-COV-001 | **Pass** | Justified exclusions only |
| SWR-AS-044 | Must | ring/isolation/pci-vf | TEST-UNIT-* | **Pass** | No business mocks |
| SWR-HVN-050 | Must | `xtask` | TEST-BUILD-001 | **Pass** | `xtask build` |
| SWR-HVN-051 | Must | `xtask` | TEST-SMOKE-001 | **Pass** | `run-nested` path used by smoke |
| SWR-HVN-052 | Must | `xtask`, `hv` | TEST-SMOKE-001 | **Pass** | `SMOKE_OK` |
| SWR-HVN-053 | Must | `xtask` | TEST-BENCH-001 | **Pass** | Bench + JSON |
| SWR-HVN-054 | Must | `xtask` | TEST-BENCH-001 | **Pass** | target miss ≠ tool fail (also verified) |
| SWR-HVN-055 | Should | lab Sriov | TEST-BENCH-001 | **Blocked** | Should: official Sriov figures — E2E gaps |
| SWR-HVN-056 | Must | `docs/HARNESS.md` | TEST-DOC-001 | **Pass** | VFIO checklist present |
| SWR-HVN-057 | Must | `xtask` | TEST-VERUS-001, TEST-COV-001 | **Pass** | verify + coverage cmds |
| SWR-HVN-058 | Must | `xtask` bench | TEST-BENCH-001 | **Pass** | Report metadata complete |
| SWR-DOC-060 | Must | `docs/system/` | TEST-DOC-001 | **Pass** | English suite |
| SWR-DOC-061 | Must | STD/STR | TEST-DOC-001 | **Pass** | This STR |
| SWR-DOC-062 | Must | ICD + suite | TEST-DOC-001 | **Pass** | CM practice |
| SWR-DOC-063 | Must | suite docs | TEST-DOC-001 | **Pass** | Explicit non-cert |

\*SWR-NIC-021: configuration **selects** Sriov when VFs vfio-bound (unit/resolve Pass). End-to-end demonstration that open yields a working L2 Sriov datapath is Blocked (ANO-001). Documented honestly — not a silent Pass.

### 9.2 Crate map (as-built)

| Area | Crate(s) |
|------|----------|
| Isolation model | `isolation` |
| Ring | `ring` |
| NIC | `pci-vf` |
| Guests / forward | `guest-wan`, `guest-lan`, `guest-common` |
| Hypervisor | `hv` |
| Config / harness | `demo-config`, `xtask` |

---

## 10. Anomalies, deviations, and waivers

| ID | Type | Description | Related TEST/SWR | Disposition | Authority |
|----|------|-------------|------------------|-------------|-----------|
| ANO-001 | Observation | Full Sriov L2 E2E (guest UART `SMOKE_OK` + Sriov bench) blocked by remaining HARNESS gaps: VF descriptor rings, Multiboot+KVM entry (some labs: KVM vCPU create fault), lab IOMMU/ACS. Software: VMLAUNCH path, guest poll, BootInfo `nic_mmio_*` now present | TEST-SMOKE-001 (Sriov), TEST-BENCH-001 (Sriov), SWR-NIC-021, SWR-HVN-055 | Fail-fast prevents false Pass; close gaps 3+5 then re-run | Programme |
| ANO-002 | Observation | TEST-ISO-002 nested EPT-violation demo Blocked (no KVM Multiboot entry here). Host unit tests cover halt-faulting-guest policy; L2 enter image is host-tested | TEST-ISO-002, SWR-ISO-003 | Re-run nested when Multiboot+KVM enters and VMXON succeeds | Programme |
| ANO-003 | Deviation | Smoke/bench use TCG Multiboot by default rather than KVM nested entry | TEST-SMOKE-001, ASM-LAB | Documented in HARNESS; L1 Inject does not need VMXON | Engineering |
| ANO-004 | Observation | Independent V&V tester not used (SYS-AS-007 recommended for governmental transfer) | Campaign §2 | Accept for spike; recommend independent re-run before external submission | Programme |

Type ∈ {Anomaly, Deviation, Waiver, Observation}.

---

## 11. Conclusions and recommendations

| Question | Answer |
|----------|--------|
| Are all Must SSS gates evidenced Pass where executable? | **Yes** for Inject/HOST campaign gates; SWR-ISO-003 nested and Sriov L2 remain Blocked (not Fail) |
| Is the suite suitable for external technical reading? | **Yes** — SSDD 0.4 as-built + STR 0.4 filled |
| Is any DAL/SIL/ASIL or airworthiness approval claimed? | **No** |
| Is the software baseline ready for demonstrator spike closure (Tasks 0–11)? | **Yes**, with Blocked items explicitly listed |
| Are SG/HAZ and rate report fields ready for transfer discussion? | **Yes**, with Inject-labelled rates and Sriov gap honesty |
| Recommendations | (1) Close spike docs. (2) Follow-on: complete L2 VMLAUNCH + Sriov MMIO before claiming Sriov E2E. (3) Re-run TEST-ISO-002 nested. (4) Optional independent V&V for authority submission. (5) Never relabel Pass as certification |

**Post-campaign conclusion:** The documentary suite is complete for the edge-hypervisor spike. Eleven of thirteen STD procedures Pass; two are Blocked on documented L2/Sriov gaps with zero Failures.

---

## 12. Approvals

| Role | Name | Organisation | Date | Signature |
|------|------|--------------|------|-----------|
| Test conductor | Spike campaign (Tasks 7–11) | AI-Pervisor | 2026-10-26 | Electronic (git) |
| Independent V&V reviewer | | | | *Pending if required for transfer* |
| Safety / airborne systems (if review intended) | | | | *Pending — review only; no cert claim* |
| Software engineering lead | | | | |
| Systems engineering lead | | | | |
| Programme / technical authority | | | | |

*Approvals beyond the spike electronic baseline are completed when this STR is issued for governmental or institutional submission.*

---

## 13. Change record

| Issue | Date | Description |
|-------|------|-------------|
| 0.1 | 2026-10-26 | Empty results skeleton |
| 0.2 | 2026-10-26 | Government-review STR template: campaign ID, environment, full SWR matrix, approvals |
| 0.3 | 2026-10-26 | Explicit non-certification; airborne rate interpretation; SWR-HVN-058/DOC-063; safety signer |
| 0.4 | 2026-10-26 | Task 11: filled campaign — Pass 11 / Fail 0 / Blocked 2; smoke/bench/coverage/Verus; SRS→crate→STD→STR matrix |
