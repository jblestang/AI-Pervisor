# STD — Software Test Description

**Product:** Hyperviseur edge gateway (spike)  
**Version:** 0.1 (Task 0 seed)  
**Language:** English

## Table of contents

1. [Purpose](#1-purpose)
2. [Test strategy](#2-test-strategy)
3. [Environment](#3-environment)
4. [Procedures](#4-procedures)
5. [Traceability matrix](#5-traceability-matrix)
6. [Pass / fail rules](#6-pass--fail-rules)

## 1. Purpose

Define executable test procedures (`TEST-*`) for SRS requirements. Each procedure maps to **`cargo xtask`** and/or `cargo test` commands. Results are recorded in [STR.md](./STR.md) after the campaign (Task 11).

## 2. Test strategy

| Layer | Command family | Mocks |
|-------|----------------|-------|
| Config / unit | `cargo test -p …` | Forbidden for ring / Inject / isolation under test |
| Proofs | `cargo xtask verify` | N/A |
| Coverage | `cargo xtask coverage` | N/A |
| Nested integration | `cargo xtask run-nested`, `smoke` | Inject only if config allows |
| Measurement | `cargo xtask bench` | Prefer Sriov for official numbers |

## 3. Environment

| Item | Requirement |
|------|-------------|
| Host | Linux with KVM; nested VT‑x (`+vmx`) |
| Toolchain | Rust per `rust-toolchain.toml`; Verus available for verify |
| Config | Complete `config/demo.toml` (from example) |
| Optional VF | IOMMU + vfio-pci per `docs/HARNESS.md` when `backend=sriov` |

Preconditions per procedure are listed below. If a lab lacks VF, Sriov-only procedures are marked **blocked** in STR (not silently skipped with a mock).

## 4. Procedures

### TEST-BUILD-001 — Workspace build

| Field | Value |
|-------|-------|
| Command | `cargo xtask build` |
| Covers | SWR-010, SWR-050 |
| Steps | 1. Ensure toolchain installed. 2. Run command. 3. Confirm `hv`, `guest-wan`, `guest-lan` images exist at `paths.*`. |
| Pass | Exit 0; images present |
| Fail | Non-zero exit or missing artifacts |

### TEST-CFG-001 — Config load and reject

| Field | Value |
|-------|-------|
| Command | `cargo test -p demo-config` |
| Covers | SWR-030, SWR-031, SWR-032, SWR-012 (indirect), IF-CFG-* |
| Steps | 1. Run package tests including incomplete-config reject and example load. 2. Optionally export env overrides and re-run override tests when present. |
| Pass | All tests green |
| Fail | Any test failure |

### TEST-UNIT-RING — Shared ring

| Field | Value |
|-------|-------|
| Command | `cargo test -p ring` |
| Covers | SWR-011, SWR-013, IF-RING-*, SWR-044 |
| Steps | 1. Run host harness tests (push/pop, full-without-overwrite). 2. Confirm no mock doubles of the ring. |
| Pass | All tests green |
| Fail | Any test failure |

### TEST-UNIT-ISO — Isolation model

| Field | Value |
|-------|-------|
| Command | `cargo test -p isolation` |
| Covers | SWR-004, SWR-001 (model level) |
| Steps | 1. Run allow/deny region tests for WAN/LAN/shared. |
| Pass | All tests green |
| Fail | Any test failure |

### TEST-UNIT-NIC — NicBackend Inject and fail-fast

| Field | Value |
|-------|-------|
| Command | `cargo test -p pci-vf` |
| Covers | SWR-020, SWR-022, SWR-023, SWR-044, IF-NIC-* |
| Steps | 1. Exercise Inject queue. 2. Assert Sriov-required + missing BDF errors. |
| Pass | All tests green |
| Fail | Any test failure |

### TEST-UNIT-FWD — Guest forward logic

| Field | Value |
|-------|-------|
| Command | `cargo test -p guest-common` (or equivalent host-tested forward crate) |
| Covers | SWR-011, SWR-014 |
| Steps | 1. Forward frames through real ring + Inject in-process. |
| Pass | All tests green |
| Fail | Any test failure / crate missing after Task 5 |

### TEST-VERUS-001 — Verus verify

| Field | Value |
|-------|-------|
| Command | `cargo xtask verify` |
| Covers | SWR-040 |
| Steps | 1. Run verify on all annotated packages. |
| Pass | Exit 0 |
| Fail | Non-zero exit (broken or missing proofs) |

### TEST-COV-001 — Coverage threshold

| Field | Value |
|-------|-------|
| Command | `cargo xtask coverage` |
| Covers | SWR-042, SWR-043 |
| Steps | 1. Run coverage. 2. Confirm line coverage ≥ 90% on declared crates. 3. Inspect allowlist for unjustified exclusions. |
| Pass | Exit 0; report ≥ 90% |
| Fail | Below threshold or silent exclusions |

### TEST-ISO-001 — EPT map builder (host unit)

| Field | Value |
|-------|-------|
| Command | `cargo test -p hv -- EPT` (or documented filter for EPT/map tests) |
| Covers | SWR-001, SWR-002, SWR-004 |
| Steps | 1. Build EPT intent from `isolation::MemoryMap` without QEMU. 2. Assert private vs shared permissions. |
| Pass | All relevant tests green |
| Fail | Any failure |

### TEST-ISO-002 — EPT violation nested

| Field | Value |
|-------|-------|
| Command | Nested lab procedure via `cargo xtask run-nested` with violation stimulus (documented in HARNESS when available) |
| Covers | SWR-003 |
| Steps | 1. Trigger out-of-region access from a guest. 2. Observe UART diagnostic and halt of faulting guest; peer integrity preserved. |
| Pass | Clean exit handling as specified |
| Fail | Silent corruption or host crash |
| Spike note | May be manual until automated; record method in STR |

### TEST-SMOKE-001 — Nested smoke

| Field | Value |
|-------|-------|
| Command | `cargo xtask smoke` |
| Covers | SWR-001, SWR-011, SWR-014, SWR-021/SWR-022, SWR-024, SWR-051, SWR-052, IF-SER-001 |
| Preconditions | `TEST-BUILD-001` pass; valid `demo.toml`; nested KVM |
| Steps | 1. Run smoke. 2. Wait for serial `SMOKE_OK`. 3. Note backend (Sriov vs Inject) from config. |
| Pass | Exit 0; `SMOKE_OK` observed |
| Fail | Timeout, non-zero exit, or missing magic |

### TEST-BENCH-001 — E2E bench @ 64 B

| Field | Value |
|-------|-------|
| Command | `cargo xtask bench` |
| Covers | SWR-053, SWR-054, SWR-055 (Should), SYS-004 |
| Preconditions | Smoke-capable lab; prefer `backend=sriov` for official figures |
| Steps | 1. Ensure `bench.frame_size = 64`. 2. Run bench. 3. Read JSON report (default `target/bench-report.json`). |
| Pass | Exit 0; report contains Gbps, pps, loss, duration, `target_met` |
| Fail | Measurement did not complete / missing report |
| Note | `target_met: false` is **not** a procedure failure |

### TEST-DOC-001 — Documentary and TCB inspection

| Field | Value |
|-------|-------|
| Command | Manual inspection (no xtask) |
| Covers | SWR-033, SWR-034, SWR-041, SWR-056, SWR-060, SWR-061 |
| Steps | 1. Confirm English suite under `docs/system/`. 2. Confirm example config, TCB list, HARNESS when those files exist. 3. Confirm STD↔SRS IDs. |
| Pass | Checklist satisfied for artifacts that exist at campaign time |
| Fail | Missing mandatory docs or hardcoding found |

## 5. Traceability matrix

| TEST ID | SWR IDs | Primary command |
|---------|---------|-----------------|
| TEST-BUILD-001 | SWR-010, SWR-050 | `cargo xtask build` |
| TEST-CFG-001 | SWR-030, SWR-031, SWR-032 | `cargo test -p demo-config` |
| TEST-UNIT-RING | SWR-011, SWR-013, SWR-044 | `cargo test -p ring` |
| TEST-UNIT-ISO | SWR-004 | `cargo test -p isolation` |
| TEST-UNIT-NIC | SWR-020, SWR-022, SWR-023, SWR-044 | `cargo test -p pci-vf` |
| TEST-UNIT-FWD | SWR-011, SWR-014 | `cargo test -p guest-common` |
| TEST-VERUS-001 | SWR-040 | `cargo xtask verify` |
| TEST-COV-001 | SWR-042, SWR-043 | `cargo xtask coverage` |
| TEST-ISO-001 | SWR-001, SWR-002, SWR-004 | `cargo test -p hv` (EPT) |
| TEST-ISO-002 | SWR-003 | `cargo xtask run-nested` (+ stimulus) |
| TEST-SMOKE-001 | SWR-001, SWR-011, SWR-014, SWR-021/022, SWR-024, SWR-051, SWR-052 | `cargo xtask smoke` |
| TEST-BENCH-001 | SWR-053, SWR-054, SWR-055 | `cargo xtask bench` |
| TEST-DOC-001 | SWR-033, SWR-034, SWR-041, SWR-056, SWR-060, SWR-061 | inspection |

## 6. Pass / fail rules

1. Automated procedures: process exit code and stated artifacts decide pass/fail.  
2. Lab-blocked Sriov cases: mark **Blocked** in STR with reason — do not substitute an undeclared mock.  
3. Bench target miss: procedure **Pass** if report valid; product objective tracked via `target_met`.  
4. Code not yet implemented (pre Task N): mark **Not run** in STR until available.

**Procedures deferred:** None beyond notes above. N/A for spike: formal DO-178 tool qualification.
