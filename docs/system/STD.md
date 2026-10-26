# Software Test Description (STD)

| Field | Value |
|-------|-------|
| **Document ID** | HV-EDGE-STD-001 |
| **Title** | Hypervisor Edge Gateway — Software Test Description |
| **Issue** | 0.2 |
| **Date** | 2026-10-26 |
| **Classification** | UNCLASSIFIED — Technology Demonstrator |
| **Parent suite** | HV-EDGE-DOC-SUITE |
| **Verifies** | HV-EDGE-SRS-001 Issue 0.2 |
| **Results recorded in** | HV-EDGE-STR-001 |

---

## Table of contents

1. [Document control](#1-document-control)  
2. [V&V strategy](#2-vv-strategy)  
3. [Environments and resources](#3-environments-and-resources)  
4. [Entry and exit criteria](#4-entry-and-exit-criteria)  
5. [Evidence and CM](#5-evidence-and-cm)  
6. [Test procedures](#6-test-procedures)  
7. [Traceability matrix](#7-traceability-matrix)  
8. [Pass, fail, blocked, not run](#8-pass-fail-blocked-not-run)  
9. [Change record](#9-change-record)

---

## 1. Document control

This STD defines **executable verification procedures** for software requirements. It is written for independent test engineers and for governmental / institutional technical reviewers who must judge objectivity of gates.

Procedures map primarily to `cargo xtask` and `cargo test` so that evidence is reproducible from a clean checkout plus documented lab prerequisites.

---

## 2. V&V strategy

| Layer | Objective | Primary commands | Mock policy |
|-------|-----------|------------------|-------------|
| Configuration | Schema integrity | `cargo test -p demo-config` | N/A |
| Component | Ring, isolation, NIC | `cargo test -p ring|isolation|pci-vf|guest-common` | **Forbidden** to mock SUT |
| Structural coverage | ≥ 90% declared crates | `cargo xtask coverage` | N/A |
| Formal specs | Verus | `cargo xtask verify` | N/A |
| Integration | Nested datapath | `cargo xtask smoke` | Inject only if config allows |
| Performance | Measurement @ 64 B | `cargo xtask bench` | Prefer Sriov for official figures |
| Inspection | Docs / hardcoding / TCB | Checklist `TEST-DOC-001` | N/A |

**Independence:** Where organisationally available, an independent tester executes smoke/bench/coverage/verify and completes STR signature blocks. Developers may execute unit procedures during implementation; STR campaign should re-run gates on a declared baseline SHA.

---

## 3. Environments and resources

### 3.1 ENV-HOST — Developer / CI host

| Resource | Requirement |
|----------|-------------|
| OS | Linux with nested virtualisation capability for integration procedures |
| Toolchain | Per `rust-toolchain.toml` (Issue 0.2: Rust **1.99.1**) |
| Verus | Per `docs/VERUS.md` when executing `TEST-VERUS-001` in hard-fail mode |
| Coverage tools | As wired by `xtask coverage` (e.g. llvm-cov) |
| Config | Complete file derived from `config/demo.example.toml` |

### 3.2 ENV-LAB-SRIOV — Official datapath lab

| Resource | Requirement |
|----------|-------------|
| All of ENV-HOST | — |
| IOMMU | Enabled; QEMU/lab configured per `docs/HARNESS.md` |
| VFs | Bound via vfio-pci; BDFs in config |
| `datapath.backend` | `sriov` |
| `allow_inject_fallback` | `false` for official figures |

### 3.3 ENV-LAB-INJECT — Degraded lab

| Resource | Requirement |
|----------|-------------|
| Nested VT‑x | Required |
| VF | Not required |
| Config | `backend=inject` **or** fallback explicitly allowed |

If ENV-LAB-SRIOV is unavailable, Sriov-specific expected results are marked **Blocked** in STR — never silently replaced by an undeclared mock.

---

## 4. Entry and exit criteria

### 4.1 Campaign entry

1. Documentary suite Issue identified (this STD + SRS Issue).  
2. Software baseline SHA recorded.  
3. Environments classified (HOST / LAB-INJECT / LAB-SRIOV).  
4. Configuration file archived with evidence pack.

### 4.2 Campaign exit

1. Every procedure in §6 has result Pass, Fail, Blocked, or Not run with rationale.  
2. Must-gate Failures dispositioned (fix or accept risk in STR).  
3. Evidence artefacts stored and linked from STR.  
4. Authority signatures completed on STR when submitted externally.

---

## 5. Evidence and CM

| Artefact | Typical path / form |
|----------|---------------------|
| Unit test logs | CI or local console capture |
| Coverage report | Path emitted by `xtask coverage` |
| Verus log | Path emitted by `xtask verify` |
| Smoke serial log | Capture including `SMOKE_OK` |
| Bench JSON | `target/bench-report.json` (or config override) |
| Config used | Copy of `demo.toml` (+ env) |
| Baseline | `git rev-parse HEAD` |

Evidence pack identifier is recorded in STR §2.

---

## 6. Test procedures

Each procedure follows: Objective → Covers → Environment → Preconditions → Steps → Expected results → Evidence.

### TEST-BUILD-001 — Workspace image build

| Field | Content |
|-------|---------|
| **Objective** | Demonstrate that hypervisor and guest images build via harness |
| **Covers** | SWR-DP-010, SWR-HVN-050 |
| **Environment** | ENV-HOST |
| **Preconditions** | Toolchain installed; repository checkout clean at baseline |
| **Steps** | 1. `cargo xtask build` 2. Verify artefacts exist at `paths.hv_image`, `paths.guest_wan_image`, `paths.guest_lan_image` |
| **Expected** | Exit code 0; three images present and non-empty |
| **Evidence** | Command log; `ls -l` of image paths |

### TEST-CFG-001 — Configuration validation

| Field | Content |
|-------|---------|
| **Objective** | Validate TOML schema accept/reject and example completeness |
| **Covers** | SWR-CFG-030, SWR-CFG-031, SWR-CFG-032, SWR-DP-012 (indirect) |
| **Environment** | ENV-HOST |
| **Steps** | 1. `cargo test -p demo-config` 2. Confirm tests cover incomplete reject and example load 3. If override tests exist, export env vars and re-run |
| **Expected** | All tests Pass |
| **Evidence** | `cargo test` output |

### TEST-UNIT-RING — SPSC ring component test

| Field | Content |
|-------|---------|
| **Objective** | Verify ring push/pop, full-without-overwrite, init rules on real memory |
| **Covers** | SWR-DP-013, SWR-DP-015, SWR-AS-044, IF-RING-* |
| **Environment** | ENV-HOST |
| **Steps** | 1. `cargo test -p ring` 2. Inspect tests: no mock of ring structure |
| **Expected** | All tests Pass |
| **Evidence** | Test output |

### TEST-UNIT-ISO — Isolation model component test

| Field | Content |
|-------|---------|
| **Objective** | Verify region allow/deny and overlap rejection |
| **Covers** | SWR-ISO-004, SWR-ISO-005 |
| **Environment** | ENV-HOST |
| **Steps** | 1. `cargo test -p isolation` |
| **Expected** | All tests Pass |
| **Evidence** | Test output |

### TEST-UNIT-NIC — NicBackend policy and Inject

| Field | Content |
|-------|---------|
| **Objective** | Verify Inject queue behaviour and Sriov fail-fast |
| **Covers** | SWR-NIC-020, SWR-NIC-022, SWR-NIC-023, SWR-AS-044 |
| **Environment** | ENV-HOST |
| **Steps** | 1. `cargo test -p pci-vf` 2. Confirm fail-fast test when Sriov required and BDF missing |
| **Expected** | All tests Pass |
| **Evidence** | Test output |
| **Applicability** | Not run until `pci-vf` tests land |

### TEST-UNIT-FWD — Forward logic

| Field | Content |
|-------|---------|
| **Objective** | Verify WAN/LAN forward using real ring + Inject in-process |
| **Covers** | SWR-DP-011, SWR-DP-014 |
| **Environment** | ENV-HOST |
| **Steps** | 1. `cargo test -p guest-common` (or documented equivalent) |
| **Expected** | All tests Pass |
| **Applicability** | Not run until guest forward extraction lands |

### TEST-VERUS-001 — Verus verification

| Field | Content |
|-------|---------|
| **Objective** | Execute specification checking on annotated crates |
| **Covers** | SWR-AS-040, SWR-HVN-057 |
| **Environment** | ENV-HOST with Verus per `docs/VERUS.md` |
| **Steps** | 1. Ensure `VERUS_BIN` or `verus` on `PATH` 2. `cargo xtask verify` 3. Interpret soft-skip vs hard-fail per VERUS.md / Task 8 policy |
| **Expected (hard-fail regime)** | Exit 0 with proofs checked |
| **Expected (stub regime)** | Soft-skip permitted only as documented; record in STR |
| **Evidence** | Verify log |

### TEST-COV-001 — Coverage threshold

| Field | Content |
|-------|---------|
| **Objective** | Enforce ≥ 90% line coverage on declared crates |
| **Covers** | SWR-AS-042, SWR-AS-043, SWR-HVN-057 |
| **Environment** | ENV-HOST |
| **Steps** | 1. `cargo xtask coverage` 2. Read report percentage 3. Inspect allowlist justifications |
| **Expected** | Exit 0; coverage ≥ 90%; no silent exclusions |
| **Evidence** | Coverage report + allowlist file |
| **Applicability** | Not run until coverage command fully wired (Task 8) |

### TEST-ISO-001 — EPT map builder (host)

| Field | Content |
|-------|---------|
| **Objective** | Verify EPT intent construction from isolation model without QEMU |
| **Covers** | SWR-ISO-001, SWR-ISO-002, SWR-ISO-004 |
| **Environment** | ENV-HOST |
| **Steps** | 1. Run `hv` unit tests for EPT/map (documented filter) |
| **Expected** | Pass; private vs shared permissions correct |
| **Applicability** | Not run until `hv` EPT tests land |

### TEST-ISO-002 — EPT violation nested

| Field | Content |
|-------|---------|
| **Objective** | Demonstrate clean handling of unauthorised access under nested execution |
| **Covers** | SWR-ISO-003 |
| **Environment** | ENV-LAB-INJECT or ENV-LAB-SRIOV |
| **Steps** | 1. Launch nested per HARNESS 2. Apply documented violation stimulus 3. Observe UART diagnostic and halt of faulting guest 4. Confirm peer memory integrity (analysis/observation) |
| **Expected** | Diagnostic present; faulting guest halted; no silent peer corruption |
| **Evidence** | Serial log + operator observation note |
| **Note** | Manual acceptable until automated; method must be recorded in STR |

### TEST-SMOKE-001 — Nested smoke

| Field | Content |
|-------|---------|
| **Objective** | End-to-end boot and datapath echo under nested QEMU |
| **Covers** | SWR-ISO-001, SWR-DP-011, SWR-DP-014, SWR-NIC-021/022, SWR-NIC-024, SWR-CFG-035, SWR-HVN-051, SWR-HVN-052 |
| **Environment** | ENV-LAB-INJECT or ENV-LAB-SRIOV |
| **Preconditions** | TEST-BUILD-001 Pass; valid config; nested KVM |
| **Steps** | 1. Archive config 2. `cargo xtask smoke` 3. Record backend in use |
| **Expected** | Exit 0; serial contains `SMOKE_OK` |
| **Evidence** | Full serial log; config copy; backend note |

### TEST-BENCH-001 — E2E measurement @ 64 B

| Field | Content |
|-------|---------|
| **Objective** | Produce mandatory performance report at 64-byte frames |
| **Covers** | SWR-HVN-053, SWR-HVN-054, SWR-HVN-055 (Should), SYS-DP-005/006 |
| **Environment** | Prefer ENV-LAB-SRIOV; else ENV-LAB-INJECT with STR annotation |
| **Preconditions** | Smoke-capable system; `bench.frame_size = 64` |
| **Steps** | 1. `cargo xtask bench` 2. Archive JSON report 3. Record `target_met` |
| **Expected** | Exit 0; report contains Gbps, pps, loss, duration, `target_met` |
| **Non-fail** | `target_met = false` is **not** a procedure failure |
| **Evidence** | `bench-report.json` |

### TEST-DOC-001 — Documentary and TCB inspection

| Field | Content |
|-------|---------|
| **Objective** | Confirm documentary suite, example config, TCB/HARNESS presence, absence of demo hardcoding |
| **Covers** | SWR-CFG-033, SWR-CFG-034, SWR-AS-041, SWR-HVN-056, SWR-DOC-060..062 |
| **Environment** | ENV-HOST (inspection) |
| **Steps** | 1. Verify `docs/system/` English suite Issue 2. Verify `config/demo.example.toml` 3. When present, verify `docs/TCB.md`, `docs/HARNESS.md` 4. Spot-check `hv`/guest sources for lab literals 5. Confirm STD↔SRS ID consistency |
| **Expected** | Checklist items satisfied for artefacts required at campaign date |
| **Evidence** | Signed checklist appendix in STR |

---

## 7. Traceability matrix

| TEST ID | SWR IDs (primary) | Command / method |
|---------|-------------------|------------------|
| TEST-BUILD-001 | SWR-DP-010, SWR-HVN-050 | `cargo xtask build` |
| TEST-CFG-001 | SWR-CFG-030..032 | `cargo test -p demo-config` |
| TEST-UNIT-RING | SWR-DP-013, SWR-DP-015, SWR-AS-044 | `cargo test -p ring` |
| TEST-UNIT-ISO | SWR-ISO-004, SWR-ISO-005 | `cargo test -p isolation` |
| TEST-UNIT-NIC | SWR-NIC-020,022,023, SWR-AS-044 | `cargo test -p pci-vf` |
| TEST-UNIT-FWD | SWR-DP-011, SWR-DP-014 | `cargo test -p guest-common` |
| TEST-VERUS-001 | SWR-AS-040, SWR-HVN-057 | `cargo xtask verify` |
| TEST-COV-001 | SWR-AS-042,043, SWR-HVN-057 | `cargo xtask coverage` |
| TEST-ISO-001 | SWR-ISO-001,002,004 | `cargo test -p hv` (EPT) |
| TEST-ISO-002 | SWR-ISO-003 | nested + stimulus |
| TEST-SMOKE-001 | SWR-ISO-001, SWR-DP-011,014, SWR-NIC-*, SWR-HVN-051,052 | `cargo xtask smoke` |
| TEST-BENCH-001 | SWR-HVN-053..055 | `cargo xtask bench` |
| TEST-DOC-001 | SWR-CFG-033,034, SWR-AS-041, SWR-HVN-056, SWR-DOC-* | inspection |

---

## 8. Pass, fail, blocked, not run

| Result | Definition |
|--------|------------|
| **Pass** | Expected results met; evidence archived |
| **Fail** | Expected results not met |
| **Blocked** | Cannot execute due to missing lab resource (e.g. VF); documented |
| **Not run** | Procedure not applicable yet on this baseline (implementation pending) |

**Rules**

1. Do not convert Blocked Sriov into Pass via undeclared mock.  
2. Bench target miss → procedure Pass if report valid.  
3. Soft-skip verify under stub regime → record explicitly; does not satisfy hard-fail gate until Task 8 policy applies.

---

## 9. Change record

| Issue | Date | Description |
|-------|------|-------------|
| 0.1 | 2026-10-26 | Skeleton |
| 0.2 | 2026-10-26 | Full STD: environments, entry/exit, evidence, detailed procedures, result vocabulary |
