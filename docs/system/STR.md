# STR — Software Test Report

**Product:** Hyperviseur edge gateway (spike)  
**Version:** 0.1 (template — fill in Task 11)  
**Language:** English

## Table of contents

1. [Purpose](#1-purpose)
2. [Campaign metadata](#2-campaign-metadata)
3. [Summary](#3-summary)
4. [Results by procedure](#4-results-by-procedure)
5. [Coverage and Verus](#5-coverage-and-verus)
6. [Bench measurement](#6-bench-measurement)
7. [Requirement traceability](#7-requirement-traceability)
8. [Anomalies and deviations](#8-anomalies-and-deviations)
9. [Conclusion](#9-conclusion)

## 1. Purpose

Record outcomes of STD procedures after smoke / bench / coverage / Verus campaigns. This file is a **template** at Task 0; populate during Task 11.

## 2. Campaign metadata

| Field | Value |
|-------|-------|
| Campaign ID | N/A for spike (pending Task 11) |
| Date | None |
| Branch / commit | None |
| Operator | None |
| Lab host | None |
| Config path | None |
| Datapath backend | None |
| Nested QEMU version | None |

## 3. Summary

| Metric | Value |
|--------|-------|
| Procedures run | 0 |
| Passed | 0 |
| Failed | 0 |
| Blocked | 0 |
| Not run | all |
| Overall spike gates | Pending |

## 4. Results by procedure

| TEST ID | Result | Evidence / notes |
|---------|--------|------------------|
| TEST-BUILD-001 | Not run | None |
| TEST-CFG-001 | Not run | None |
| TEST-UNIT-RING | Not run | None |
| TEST-UNIT-ISO | Not run | None |
| TEST-UNIT-NIC | Not run | None |
| TEST-UNIT-FWD | Not run | None |
| TEST-VERUS-001 | Not run | None |
| TEST-COV-001 | Not run | None |
| TEST-ISO-001 | Not run | None |
| TEST-ISO-002 | Not run | None |
| TEST-SMOKE-001 | Not run | None |
| TEST-BENCH-001 | Not run | None |
| TEST-DOC-001 | Not run | None |

Result vocabulary: `Pass` | `Fail` | `Blocked` | `Not run`.

## 5. Coverage and Verus

| Item | Value |
|------|-------|
| Coverage report path | None |
| Line coverage % | N/A for spike |
| Threshold (90%) met | N/A for spike |
| Allowlist path | None |
| Verus log path | None |
| Verus outcome | N/A for spike |

## 6. Bench measurement

| Field | Value |
|-------|-------|
| Report path | None (expected `target/bench-report.json`) |
| Frame size | N/A for spike |
| Duration (s) | N/A for spike |
| Bidirectional | N/A for spike |
| Gbps | N/A for spike |
| pps | N/A for spike |
| Loss | N/A for spike |
| `target_gbps` | N/A for spike |
| `target_met` | N/A for spike |

## 7. Requirement traceability

Fill after campaign: SRS ID → crate(s) → TEST ID → STR result.

| SWR ID | Crate(s) | TEST ID | Result |
|--------|----------|---------|--------|
| None | None | None | None |

Extend rows for all `SWR-*` in [SRS.md](./SRS.md) during Task 11.

## 8. Anomalies and deviations

| ID | Description | Impact | Disposition |
|----|-------------|--------|-------------|
| None | None | None | None |

## 9. Conclusion

Pending Task 11. Spike success criteria from SSS §8 are **not** claimed until this report is filled with Pass evidence for the Must-gated procedures.
