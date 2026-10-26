# Software Test Report (STR)

| Field | Value |
|-------|-------|
| **Document ID** | HV-EDGE-STR-001 |
| **Title** | Hypervisor Edge Gateway — Software Test Report |
| **Issue** | 0.2 (template) |
| **Date** | 2026-10-26 |
| **Classification** | UNCLASSIFIED — Technology Demonstrator |
| **Parent suite** | HV-EDGE-DOC-SUITE |
| **Procedures** | HV-EDGE-STD-001 Issue 0.2 |
| **Requirements** | HV-EDGE-SRS-001 Issue 0.2 |
| **Status** | **TEMPLATE** — populate during / after verification campaign (Task 11) |

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
9. [Requirement verification summary](#9-requirement-verification-summary)  
10. [Anomalies, deviations, and waivers](#10-anomalies-deviations-and-waivers)  
11. [Conclusions and recommendations](#11-conclusions-and-recommendations)  
12. [Approvals](#12-approvals)  
13. [Change record](#13-change-record)

---

## 1. Purpose and limitations

This report records the **outcome of the STD verification campaign** against a declared software baseline. It is intended for programme management and external technical authority review.

**Limitations (standing):**

- Demonstrator under nested QEMU/KVM is not a bare-metal certification campaign.  
- Should-priority performance (1 Gbps @ 64 B) may be unmet while Must-priority measurement still Passes.  
- Procedures marked Not run indicate incomplete vertical slice on the tested baseline — not silent omission.

Until campaign execution, all quantitative fields remain **None** / **N/A for spike** as placeholders without TBD narrative spam.

---

## 2. Campaign identification

| Field | Value |
|-------|-------|
| Campaign ID | None |
| Campaign name | None |
| Execution start (Europe/Paris) | None |
| Execution end (Europe/Paris) | None |
| STD Issue | HV-EDGE-STD-001 Issue 0.2 |
| SRS Issue | HV-EDGE-SRS-001 Issue 0.2 |
| SSS/SSDD Issue | 0.2 |
| Git repository | `https://github.com/jblestang/AI-Pervisor` |
| Git branch | None |
| Git commit SHA | None |
| Evidence pack location | None |
| Operator / independent tester | None |
| Organisation | None |

---

## 3. Executive summary

| Metric | Value |
|--------|-------|
| Procedures planned | 13 |
| Pass | 0 |
| Fail | 0 |
| Blocked | 0 |
| Not run | 13 |
| Must-gate status (SSS §10) | Pending |
| Should-gate (1 Gbps) | Pending / N/A for spike |
| Recommendation | **Do not claim demonstrator closure** until Must gates Pass |

**Narrative (post-campaign):** None

---

## 4. Environment as tested

| Item | Value |
|------|-------|
| Environment class | None (HOST / LAB-INJECT / LAB-SRIOV) |
| Host CPU / model | None |
| Nested VT‑x confirmed | None |
| QEMU version | None |
| Kernel / KVM | None |
| IOMMU | None |
| WAN VF BDF | None |
| LAN VF BDF | None |
| `datapath.backend` | None |
| `allow_inject_fallback` | None |
| Config file hash/path | None |
| Rust toolchain | None |
| Verus version | None |

---

## 5. Results by procedure

| TEST ID | Result | Date | Evidence reference | Notes |
|---------|--------|------|--------------------|-------|
| TEST-BUILD-001 | Not run | None | None | None |
| TEST-CFG-001 | Not run | None | None | None |
| TEST-UNIT-RING | Not run | None | None | As-built tests may exist; campaign re-run pending |
| TEST-UNIT-ISO | Not run | None | None | As-built tests may exist; campaign re-run pending |
| TEST-UNIT-NIC | Not run | None | None | None |
| TEST-UNIT-FWD | Not run | None | None | None |
| TEST-VERUS-001 | Not run | None | None | See `docs/VERUS.md` stub regime |
| TEST-COV-001 | Not run | None | None | None |
| TEST-ISO-001 | Not run | None | None | None |
| TEST-ISO-002 | Not run | None | None | None |
| TEST-SMOKE-001 | Not run | None | None | None |
| TEST-BENCH-001 | Not run | None | None | None |
| TEST-DOC-001 | Not run | None | None | Issue 0.2 suite present; formal checklist pending |

Result vocabulary: **Pass** | **Fail** | **Blocked** | **Not run** (per STD §8).

---

## 6. Coverage results

| Item | Value |
|------|-------|
| Command | `cargo xtask coverage` |
| Report path | None |
| Declared crates | None |
| Line coverage % | N/A for spike |
| Threshold | 90% |
| Threshold met | N/A for spike |
| Allowlist path | None |
| Allowlist review | None |

---

## 7. Verus results

| Item | Value |
|------|-------|
| Command | `cargo xtask verify` |
| Log path | None |
| Crates checked | None |
| Regime | stub soft-skip / hard-fail (select one at campaign) |
| Outcome | N/A for spike |
| Non-conformances | None |

---

## 8. Bench measurement results

| Field | Value |
|-------|-------|
| Report path | None |
| Frame size (B) | N/A for spike |
| Duration (s) | N/A for spike |
| Bidirectional | N/A for spike |
| Backend used | N/A for spike |
| Gbps | N/A for spike |
| pps | N/A for spike |
| Loss | N/A for spike |
| `target_gbps` | N/A for spike |
| `target_met` | N/A for spike |
| Procedure result | Not run |

---

## 9. Requirement verification summary

Populate one row per `SWR-*` after campaign. Until then the table records structure only.

| SWR ID | Priority | TEST ID(s) | Result | Remarks |
|--------|----------|------------|--------|---------|
| SWR-ISO-001 | Must | TEST-ISO-001, TEST-SMOKE-001 | Not run | None |
| SWR-ISO-002 | Must | TEST-ISO-001 | Not run | None |
| SWR-ISO-003 | Must | TEST-ISO-002 | Not run | None |
| SWR-ISO-004 | Must | TEST-UNIT-ISO, TEST-VERUS-001 | Not run | None |
| SWR-ISO-005 | Must | TEST-UNIT-ISO | Not run | None |
| SWR-DP-010 | Must | TEST-BUILD-001 | Not run | None |
| SWR-DP-011 | Must | TEST-SMOKE-001, TEST-UNIT-FWD | Not run | None |
| SWR-DP-012 | Must | TEST-CFG-001, TEST-DOC-001 | Not run | None |
| SWR-DP-013 | Must | TEST-UNIT-RING | Not run | None |
| SWR-DP-014 | Must | TEST-SMOKE-001, TEST-BENCH-001 | Not run | None |
| SWR-DP-015 | Must | TEST-UNIT-RING, TEST-VERUS-001 | Not run | None |
| SWR-NIC-020 | Must | TEST-UNIT-NIC | Not run | None |
| SWR-NIC-021 | Must | TEST-SMOKE-001 | Not run | None |
| SWR-NIC-022 | Must | TEST-UNIT-NIC | Not run | None |
| SWR-NIC-023 | Must | TEST-UNIT-NIC | Not run | None |
| SWR-NIC-024 | Must | TEST-DOC-001, TEST-SMOKE-001 | Not run | None |
| SWR-CFG-030 | Must | TEST-CFG-001 | Not run | None |
| SWR-CFG-031 | Must | TEST-CFG-001 | Not run | None |
| SWR-CFG-032 | Must | TEST-CFG-001 | Not run | None |
| SWR-CFG-033 | Must | TEST-DOC-001 | Not run | None |
| SWR-CFG-034 | Must | TEST-DOC-001 | Not run | None |
| SWR-CFG-035 | Must | TEST-SMOKE-001 | Not run | None |
| SWR-AS-040 | Must | TEST-VERUS-001 | Not run | None |
| SWR-AS-041 | Must | TEST-DOC-001 | Not run | None |
| SWR-AS-042 | Must | TEST-COV-001 | Not run | None |
| SWR-AS-043 | Must | TEST-COV-001 | Not run | None |
| SWR-AS-044 | Must | TEST-UNIT-* | Not run | None |
| SWR-HVN-050 | Must | TEST-BUILD-001 | Not run | None |
| SWR-HVN-051 | Must | TEST-SMOKE-001 | Not run | None |
| SWR-HVN-052 | Must | TEST-SMOKE-001 | Not run | None |
| SWR-HVN-053 | Must | TEST-BENCH-001 | Not run | None |
| SWR-HVN-054 | Must | TEST-BENCH-001 | Not run | None |
| SWR-HVN-055 | Should | TEST-BENCH-001 | Not run | None |
| SWR-HVN-056 | Must | TEST-DOC-001 | Not run | None |
| SWR-HVN-057 | Must | TEST-VERUS-001, TEST-COV-001 | Not run | None |
| SWR-DOC-060 | Must | TEST-DOC-001 | Not run | None |
| SWR-DOC-061 | Must | TEST-DOC-001 | Not run | None |
| SWR-DOC-062 | Must | TEST-DOC-001 | Not run | None |

**Crate mapping (as-built / intended)** — complete at Task 11:

| Area | Crate(s) |
|------|----------|
| Isolation model | `isolation` |
| Ring | `ring` |
| NIC | `pci-vf` |
| Guests | `guest-wan`, `guest-lan`, `guest-common` |
| Hypervisor | `hv` |
| Config / harness | `demo-config`, `xtask` |

---

## 10. Anomalies, deviations, and waivers

| ID | Type | Description | Related TEST/SWR | Disposition | Authority |
|----|------|-------------|------------------|-------------|-----------|
| None | — | None | None | None | None |

Type ∈ {Anomaly, Deviation, Waiver, Observation}.

---

## 11. Conclusions and recommendations

| Question | Answer (pre-campaign) |
|----------|------------------------|
| Are all Must SSS gates evidenced Pass? | No — campaign not executed |
| Is the suite suitable for external technical reading? | Yes — Issue 0.2 documentary baseline |
| Is the software baseline ready for demonstrator closure? | No — pending STD execution |
| Recommendations | Execute campaign per STD; fill §§2–10; obtain approvals in §12 |

**Post-campaign conclusion:** None

---

## 12. Approvals

| Role | Name | Organisation | Date | Signature |
|------|------|--------------|------|-----------|
| Test conductor | | | | |
| Independent V&V reviewer | | | | |
| Software engineering lead | | | | |
| Systems engineering lead | | | | |
| Programme / technical authority | | | | |

*Approvals are completed when this STR is issued for governmental or institutional submission.*

---

## 13. Change record

| Issue | Date | Description |
|-------|------|-------------|
| 0.1 | 2026-10-26 | Empty results skeleton |
| 0.2 | 2026-10-26 | Government-review STR template: campaign ID, environment, full SWR matrix, approvals |
