# SRS — Software Requirements Specification

**Product:** Hyperviseur edge gateway (spike)  
**Version:** 0.1 (Task 0 seed from design)  
**Language:** English

## Table of contents

1. [Purpose](#1-purpose)
2. [Conventions](#2-conventions)
3. [Isolation and memory](#3-isolation-and-memory)
4. [Datapath and guests](#4-datapath-and-guests)
5. [NIC backends](#5-nic-backends)
6. [Configuration](#6-configuration)
7. [Proofs and coverage](#7-proofs-and-coverage)
8. [Harness and measurement](#8-harness-and-measurement)
9. [Documentation](#9-documentation)
10. [Requirement index](#10-requirement-index)

## 1. Purpose

State **testable** software requirements (`SWR-*`) derived from the spike design. Acceptance is demonstrated by STD procedures (`TEST-*`) and recorded in STR.

## 2. Conventions

| Field | Meaning |
|-------|---------|
| ID | Stable `SWR-NNN` |
| Priority | Must (spike gate) / Should (objective) |
| Verification | Primary method (test / proof / inspection / measurement) |

Priority **Must** items block spike success criteria. **Should** items are reported even if unmet (e.g. 1 Gbps target).

## 3. Isolation and memory

| ID | Requirement | Priority | Verification |
|----|-------------|----------|--------------|
| SWR-001 | `hv` shall map private WAN and private LAN guest memory as mutually inaccessible via EPT. | Must | TEST-ISO-001, TEST-SMOKE-001 |
| SWR-002 | Shared ring (and agreed shared pages) shall be mapped read-write to both guests and nowhere else that would break isolation. | Must | TEST-ISO-001 |
| SWR-003 | An EPT violation for out-of-region access shall cause a handled exit: UART diagnostic and halt of the faulting guest without silently corrupting peer memory. | Must | TEST-ISO-002 |
| SWR-004 | The Rust `isolation` crate shall expose a region/permission model used when recording EPT intent; access checks shall reject overlapping private WAN/LAN claims. | Must | TEST-UNIT-ISO, TEST-VERUS-001 |

## 4. Datapath and guests

| ID | Requirement | Priority | Verification |
|----|-------------|----------|--------------|
| SWR-010 | WAN and LAN guests shall be Rust `no_std` (+ `alloc`) binaries loaded by `hv`. | Must | TEST-BUILD-001, inspection |
| SWR-011 | Guests shall forward frames WAN→ring→LAN and LAN→ring→WAN using the shared ring ABI (`IF-RING-*`). | Must | TEST-SMOKE-001, TEST-UNIT-FWD |
| SWR-012 | Guests shall obtain ring base, arena, NIC, UART, and stats locations from the boot info blob (`IF-BOOT-*`), not from compile-time lab literals. | Must | TEST-CFG-001, inspection |
| SWR-013 | When the shared ring is full, the producer shall drop the frame and increment a drop counter; it shall not block indefinitely. | Must | TEST-UNIT-RING |
| SWR-014 | Guests shall update a shared stats page (`IF-STATS-*`) consumed by smoke/bench. | Must | TEST-SMOKE-001, TEST-BENCH-001 |

## 5. NIC backends

| ID | Requirement | Priority | Verification |
|----|-------------|----------|--------------|
| SWR-020 | `pci-vf` shall expose a single `NicBackend` API for receive/send used by both guests. | Must | TEST-UNIT-NIC |
| SWR-021 | When config selects `sriov` and the VF is present, the backend shall be `Sriov`. | Must | TEST-SMOKE-001 (lab), HARNESS |
| SWR-022 | When config selects `inject`, or (`allow_inject_fallback` and VF absent), the backend shall be `Inject` (in-process frame queue). | Must | TEST-UNIT-NIC |
| SWR-023 | When config requires `sriov` and the device/BDF is missing, open shall fail fast with an error — no silent substitute. | Must | TEST-UNIT-NIC |
| SWR-024 | Switching between `Sriov` and `Inject` shall require config changes only; guest forward logic shall remain unchanged. | Must | inspection, TEST-SMOKE-001 |

## 6. Configuration

| ID | Requirement | Priority | Verification |
|----|-------------|----------|--------------|
| SWR-030 | Demonstrator parameters shall be loaded from TOML conforming to `IF-CFG-*` (lab, datapath, guests, ring, bench, paths). | Must | TEST-CFG-001 |
| SWR-031 | Incomplete TOML shall be rejected with an error naming the missing field. | Must | TEST-CFG-001 |
| SWR-032 | Environment overrides `HV_DEMO_CONFIG`, `HV_VF_BACKEND`, `HV_WAN_VF_BDF`, `HV_LAN_VF_BDF` shall apply as specified in ICD. | Must | TEST-CFG-001 |
| SWR-033 | Source code shall not embed lab-specific BDFs, memory sizes, ring capacities, or bench durations as demonstrator defaults. | Must | inspection |
| SWR-034 | An example complete config (`config/demo.example.toml`) shall be checked into the repository. | Must | inspection |

## 7. Proofs and coverage

| ID | Requirement | Priority | Verification |
|----|-------------|----------|--------------|
| SWR-040 | All provable safe Rust in `ring`, `isolation`, guest datapath logic, and safe parts of `pci-vf` shall carry Verus specs; `cargo xtask verify` shall fail if proofs break. | Must | TEST-VERUS-001 |
| SWR-041 | Every `unsafe` block in the TCB shall be listed in `docs/TCB.md` with a one-line justification. | Must | inspection |
| SWR-042 | `cargo xtask coverage` shall measure line coverage on the declared crate set and **fail** if coverage &lt; 90%. | Must | TEST-COV-001 |
| SWR-043 | Coverage exclusions shall be limited to listed TCB/glue files with explicit justification in `xtask/coverage-allowlist.toml`. | Must | inspection |
| SWR-044 | Unit tests shall exercise real ring, Inject queue, and isolation structures — not mocks of those components. | Must | inspection, unit tests |

## 8. Harness and measurement

| ID | Requirement | Priority | Verification |
|----|-------------|----------|--------------|
| SWR-050 | Operators shall build images via `cargo xtask build`. | Must | TEST-BUILD-001 |
| SWR-051 | `cargo xtask run-nested` shall launch QEMU nested with args derived only from `DemoConfig`. | Must | TEST-SMOKE-001 |
| SWR-052 | `cargo xtask smoke` shall wait for serial magic `SMOKE_OK` after a successful inject/sriov echo path. | Must | TEST-SMOKE-001 |
| SWR-053 | `cargo xtask bench` shall run for `bench.duration_secs` at `bench.frame_size` (default 64), bidirectional when configured, and write a JSON report (Gbps, pps, loss, `target_met`). | Must | TEST-BENCH-001 |
| SWR-054 | Bench shall exit 0 when measurement completes even if `target_gbps` is not met; the report shall set `target_met: false` in that case. | Must | TEST-BENCH-001 |
| SWR-055 | Official bench / production demo path should use `Sriov` + real VFs when the lab provides them. | Should | TEST-BENCH-001 (lab) |
| SWR-056 | `docs/HARNESS.md` shall document VFIO binding and nested SR‑IOV setup. | Must | inspection |

## 9. Documentation

| ID | Requirement | Priority | Verification |
|----|-------------|----------|--------------|
| SWR-060 | The documentary suite SSS/SSDD/SRS/ICD/STD/STR shall remain in English under `docs/system/`. | Must | inspection |
| SWR-061 | STD procedures shall reference `SWR-*` IDs; STR shall record results per `TEST-*`. | Must | inspection |

## 10. Requirement index

| ID | Title (short) |
|----|---------------|
| SWR-001 | EPT private WAN/LAN |
| SWR-002 | Shared ring mapping |
| SWR-003 | EPT violation handling |
| SWR-004 | Isolation model crate |
| SWR-010 | no_std guests |
| SWR-011 | Bidirectional ring forward |
| SWR-012 | Boot-info driven layout |
| SWR-013 | Ring-full drop |
| SWR-014 | Stats page |
| SWR-020 | NicBackend API |
| SWR-021 | Sriov when present |
| SWR-022 | Inject when allowed |
| SWR-023 | Sriov fail-fast |
| SWR-024 | Config-only backend switch |
| SWR-030 | TOML DemoConfig |
| SWR-031 | Reject incomplete config |
| SWR-032 | Env overrides |
| SWR-033 | No demo hardcoding |
| SWR-034 | Example config checked in |
| SWR-040 | Verus on provable Rust |
| SWR-041 | TCB documentation |
| SWR-042 | Coverage ≥ 90% |
| SWR-043 | Explicit coverage allowlist |
| SWR-044 | No business mocks |
| SWR-050 | xtask build |
| SWR-051 | xtask run-nested |
| SWR-052 | xtask smoke |
| SWR-053 | xtask bench report |
| SWR-054 | Bench exit vs target |
| SWR-055 | Prefer Sriov for official bench |
| SWR-056 | HARNESS.md |
| SWR-060 | English doc suite |
| SWR-061 | Traceability STD/STR |

**Deferred / N/A for spike:** bare-metal boot requirements — None in this SRS.
