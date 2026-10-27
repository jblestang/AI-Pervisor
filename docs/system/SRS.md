# Software Requirements Specification (SRS)

| Field | Value |
|-------|-------|
| **Document ID** | HV-EDGE-SRS-001 |
| **Title** | Hypervisor Edge Gateway — Software Requirements Specification |
| **Issue** | 0.3 |
| **Date** | 2026-10-26 |
| **Classification** | UNCLASSIFIED — Technology Demonstrator |
| **Parent suite** | HV-EDGE-DOC-SUITE |
| **Parents** | HV-EDGE-SSS-001 Issue 0.3, HV-EDGE-SSDD-001, HV-EDGE-ICD-001 |
| **Transfer context** | Safety-related; airborne rates (not DAL/SIL certified) |

---

## Table of contents

1. [Document control](#1-document-control)  
2. [Scope](#2-scope)  
3. [Requirement attributes](#3-requirement-attributes)  
4. [Isolation and memory management](#4-isolation-and-memory-management)  
5. [Datapath and guests](#5-datapath-and-guests)  
6. [NIC backends](#6-nic-backends)  
7. [Configuration and build](#7-configuration-and-build)  
8. [Assurance (proofs and coverage)](#8-assurance-proofs-and-coverage)  
9. [Harness and measurement](#9-harness-and-measurement)  
10. [Documentation and CM](#10-documentation-and-cm)  
11. [Requirements index and parent traceability](#11-requirements-index-and-parent-traceability)  
12. [Change record](#12-change-record)

---

## 1. Document control

This SRS states **software requirements** for the demonstrator. Each requirement is uniquely identified, prioritised, and assigned a verification method. Interface field meanings are owned by the ICD; this SRS requires software to **conform** to those interfaces.

### 1.1 Requirement language

Same as SSS: **shall** / **should** / **may** / **will**.

### 1.2 Verification methods

| Code | Method | Meaning |
|------|--------|---------|
| **T** | Test | Automated or scripted procedure with expected result |
| **D** | Demonstration | Observed behaviour on laboratory configuration |
| **A** | Analysis | Proof, model check, or reasoned calculation |
| **I** | Inspection | Document/code review checklist |

---

## 2. Scope

**In scope:** software in packages `hv`, `guest-wan`, `guest-lan`, optional `guest-common`, `ring`, `isolation`, `pci-vf`, `demo-config`, `xtask`, and associated tests/docs hooks.

**Out of scope:** L0 host OS, QEMU internals, NIC silicon firmware, governmental process certification of the development environment.

---

## 3. Requirement attributes

Each requirement table uses:

| Column | Content |
|--------|---------|
| ID | Stable `SWR-<AREA>-<nnn>` |
| Requirement | Shall statement |
| Priority | Must / Should |
| Parent | `SYS-*`, `SG-*`, and/or `IF-*` |
| Sig. | Safety significance for transfer: **SR** (safety-relevant), **NSR** (not safety-relevant), **SUP** (supports safety argument / evidence only) |
| V&V | Method codes |
| Procedure | Primary `TEST-*` |

> **Sig. is not a DAL/SIL.** It flags which requirements a follow-on safety/airborne programme would likely carry forward.

---

## 4. Isolation and memory management

| ID | Requirement | Priority | Parent | Sig. | V&V | Procedure |
|----|-------------|----------|--------|------|-----|-----------|
| SWR-ISO-001 | The hypervisor software **shall** construct EPT mappings such that WAN private guest memory is not readable or writable by the LAN guest, and LAN private guest memory is not readable or writable by the WAN guest. | Must | SYS-ISO-001, SG-ISO-001 | SR | T/D | TEST-ISO-001, TEST-SMOKE-001 |
| SWR-ISO-002 | The hypervisor software **shall** map shared ring headers, descriptor slots, payload arena, and stats page with read-write access for both guests as specified in ICD `IF-BOOT-*` / `IF-RING-*` / `IF-STATS-*`. | Must | SYS-ISO-002, SG-ISO-002, IF-BOOT-002 | SR | T/I | TEST-ISO-001 |
| SWR-ISO-003 | On an EPT violation attributable to unauthorised access, hypervisor software **shall** (a) emit a diagnostic on the UART path, (b) halt the faulting guest, and (c) refrain from applying the faulting write to peer private memory. | Must | SYS-ISO-003, SG-FCC-001 | SR | T/D | TEST-ISO-002 |
| SWR-ISO-004 | The `isolation` software **shall** provide a region and permission model with an access-check operation; insertion of overlapping regions that would allow conflicting private WAN/LAN claims **shall** be rejected. | Must | SYS-ISO-004, SG-ISO-001 | SR | T/A | TEST-UNIT-ISO, TEST-VERUS-001 |
| SWR-ISO-005 | Permission bits in the isolation model **shall** distinguish at least read, write, and execute rights, and access checks **shall** deny requests lacking required rights. | Must | SYS-ISO-004 | SR | T | TEST-UNIT-ISO |

---

## 5. Datapath and guests

| ID | Requirement | Priority | Parent | Sig. | V&V | Procedure |
|----|-------------|----------|--------|------|-----|-----------|
| SWR-DP-010 | WAN and LAN guest software **shall** be built as Rust `no_std` (+ `alloc`) images loadable by the hypervisor. | Must | SYS-DP-001 | NSR | T/I | TEST-BUILD-001 |
| SWR-DP-011 | Guest software **shall** forward frames in both directions WAN→ring→LAN and LAN→ring→WAN using the shared ring ABI (`IF-RING-*`). | Must | SYS-DP-001, IF-RING-003 | SR | T | TEST-SMOKE-001, TEST-UNIT-FWD |
| SWR-DP-012 | Guest software **shall** obtain ring, arena, NIC, UART, and stats locations exclusively from the boot information blob (`IF-BOOT-*`); guest source **shall not** embed laboratory-specific addresses or sizes as demonstrator defaults. | Must | SYS-AS-003, SG-CFG-001, IF-BOOT-* | SR | T/I | TEST-CFG-001, TEST-DOC-001 |
| SWR-DP-013 | When `try_push` indicates ring full, producer software **shall** drop the frame and increment the drop counter defined in `IF-STATS-*`; software **shall not** block indefinitely awaiting space. | Must | IF-RING-003, IF-STATS-001, SG-ISO-002 | SR | T | TEST-UNIT-RING |
| SWR-DP-014 | Guest software **shall** update the shared statistics page counters defined in `IF-STATS-001` so that smoke and bench procedures can observe progress. | Must | SYS-DP-005, IF-STATS-001 | SUP | T | TEST-SMOKE-001, TEST-BENCH-001 |
| SWR-DP-015 | Ring software **shall** implement SPSC semantics: capacity a power of two; full and empty conditions per ICD; descriptor format `FrameDesc { offset: u32, len: u32 }` (`IF-RING-001`). | Must | IF-RING-001..003 | SR | T/A | TEST-UNIT-RING, TEST-VERUS-001 |

---

## 6. NIC backends

| ID | Requirement | Priority | Parent | Sig. | V&V | Procedure |
|----|-------------|----------|--------|------|-----|-----------|
| SWR-NIC-020 | `pci-vf` software **shall** expose a single `NicBackend` interface providing at least readiness query, receive into a buffer, and transmit of a frame (`IF-NIC-001`). | Must | SYS-DP-002, IF-NIC-001 | NSR | T | TEST-UNIT-NIC |
| SWR-NIC-021 | When configuration selects backend `sriov` and the configured VF is present, open **shall** yield the Sriov backend. | Must | SYS-DP-002, IF-NIC-003 | SUP | D/T | TEST-SMOKE-001 |
| SWR-NIC-022 | When configuration selects `inject`, or when `allow_inject_fallback` is true and no VF is available, open **shall** yield the Inject backend (in-process frame queue). | Must | SYS-DP-003, IF-NIC-003 | SUP | T | TEST-UNIT-NIC |
| SWR-NIC-023 | When configuration requires `sriov` with `allow_inject_fallback = false` and the VF/BDF is missing, open **shall** return an error and **shall not** silently substitute another backend. | Must | SYS-DP-004, SG-CFG-001, IF-NIC-003 | SR | T | TEST-UNIT-NIC |
| SWR-NIC-024 | Guest forward logic **shall** depend only on `NicBackend`; switching Sriov↔Inject **shall** require configuration change only, not guest source change. | Must | SYS-DP-003 | SUP | I/D | TEST-DOC-001, TEST-SMOKE-001 |

---

## 7. Configuration and build

| ID | Requirement | Priority | Parent | Sig. | V&V | Procedure |
|----|-------------|----------|--------|------|-----|-----------|
| SWR-CFG-030 | Configuration software **shall** load demonstrator parameters from TOML sections defined by ICD `IF-CFG-*` (lab, datapath, guests, ring, bench, paths). | Must | SYS-AS-003, IF-CFG-* | SR | T | TEST-CFG-001 |
| SWR-CFG-031 | Configuration software **shall** reject incomplete TOML with an error that identifies the missing mandatory field. | Must | SYS-AS-003, SG-CFG-001, IF-CFG-REJECT | SR | T | TEST-CFG-001 |
| SWR-CFG-032 | Configuration software **shall** honour environment overrides `HV_DEMO_CONFIG`, `HV_VF_BACKEND`, `HV_WAN_VF_BDF`, and `HV_LAN_VF_BDF` as specified in `IF-CFG-ENV`. | Must | IF-CFG-ENV | SUP | T | TEST-CFG-001 |
| SWR-CFG-033 | Application source for `hv` and guests **shall not** embed laboratory-specific BDFs, memory sizes, ring capacities, or bench durations as demonstrator defaults. | Must | SYS-AS-003, SG-CFG-001 | SR | I | TEST-DOC-001 |
| SWR-CFG-034 | The repository **shall** contain a complete example configuration file `config/demo.example.toml` conforming to `IF-CFG-*`. | Must | SYS-AS-003 | SUP | I | TEST-DOC-001 |
| SWR-CFG-035 | QEMU command-line construction in the harness **shall** derive machine parameters solely from `DemoConfig` (no parallel hardcoded lab profile). | Must | SYS-AS-003, SYS-AS-004 | SUP | I/T | TEST-SMOKE-001 |

---

## 8. Assurance (proofs and coverage)

| ID | Requirement | Priority | Parent | Sig. | V&V | Procedure |
|----|-------------|----------|--------|------|-----|-----------|
| SWR-AS-040 | Provable safe software in `ring`, `isolation`, guest datapath logic, and safe portions of `pci-vf` **shall** carry Verus specifications; `cargo xtask verify` **shall** fail when proofs are broken under the hard-fail regime defined for Task 8 / `docs/VERUS.md`. | Must | SYS-AS-001 | SUP | T/A | TEST-VERUS-001 |
| SWR-AS-041 | Every `unsafe` block belonging to the TCB **shall** be listed in `docs/TCB.md` with a one-line justification. | Must | SYS-AS-005, SG-ISO-001 | SR | I | TEST-DOC-001 |
| SWR-AS-042 | `cargo xtask coverage` **shall** measure line coverage on the declared crate set and **shall** exit unsuccessful if coverage is strictly less than 90%. | Must | SYS-AS-002 | SUP | T | TEST-COV-001 |
| SWR-AS-043 | Coverage exclusions **shall** be limited to files listed in an allowlist with explicit justification; silent exclusion **shall not** be used. | Must | SYS-AS-002 | SUP | I | TEST-COV-001 |
| SWR-AS-044 | Unit tests for ring, isolation, and Inject **shall** exercise the real implementations; introduction of mocks that replace those components under test **shall** be treated as non-compliant. | Must | CON-QA-001 | SR | I/T | TEST-UNIT-RING, TEST-UNIT-ISO, TEST-UNIT-NIC |

---

## 9. Harness and measurement

| ID | Requirement | Priority | Parent | Sig. | V&V | Procedure |
|----|-------------|----------|--------|------|-----|-----------|
| SWR-HVN-050 | The software **shall** provide `cargo xtask build` producing hypervisor and guest images at paths indicated by configuration. | Must | SYS-AS-004 | NSR | T | TEST-BUILD-001 |
| SWR-HVN-051 | The software **shall** provide `cargo xtask run-nested` launching nested QEMU using arguments derived from `DemoConfig`. | Must | SYS-AS-004 | SUP | D/T | TEST-SMOKE-001 |
| SWR-HVN-052 | The software **shall** provide `cargo xtask smoke` that waits for the serial magic string `SMOKE_OK` (`IF-SER-001`) after a successful datapath exercise. | Must | SYS-DP-001, IF-SER-001 | SUP | T | TEST-SMOKE-001 |
| SWR-HVN-053 | The software **shall** provide `cargo xtask bench` that runs for `bench.duration_secs` at `bench.frame_size` (demonstrator default 64), bidirectional when configured, and writes a JSON report including Gbps, pps, loss, duration, and `target_met`. | Must | SYS-DP-005, SG-RATE-001, IF-CFG-BENCH | SUP | T | TEST-BENCH-001 |
| SWR-HVN-054 | Bench **shall** exit successfully when measurement completes even if configured `target_gbps` is not met; the report **shall** set `target_met` to false in that case. | Must | SYS-DP-006, SG-RATE-001 | SR | T | TEST-BENCH-001 |
| SWR-HVN-055 | For official published performance figures (including those shown to airborne stakeholders), the configuration **should** select `sriov` with real VFs. | Should | SYS-DP-006, SG-RATE-001 | SUP | D | TEST-BENCH-001 |
| SWR-HVN-056 | Repository documentation **shall** include `docs/HARNESS.md` describing VFIO binding and nested SR‑IOV setup sufficient to switch backend without guest code changes. | Must | SYS-AS-004 | SUP | I | TEST-DOC-001 |
| SWR-HVN-057 | The software **shall** provide `cargo xtask verify` and `cargo xtask coverage` entry points for assurance gates. | Must | SYS-AS-001, SYS-AS-002 | SUP | T | TEST-VERUS-001, TEST-COV-001 |
| SWR-HVN-058 | The bench JSON report **shall** identify at least: frame size, duration, bidirectional flag, datapath backend, Gbps, pps, loss, `target_gbps`, and `target_met` (SYS-DP-007). | Must | SYS-DP-007, SG-RATE-001 | SUP | T/I | TEST-BENCH-001 |

---

## 10. Documentation and CM

| ID | Requirement | Priority | Parent | Sig. | V&V | Procedure |
|----|-------------|----------|--------|------|-----|-----------|
| SWR-DOC-060 | The documentary suite SSS, SSDD, SRS, ICD, STD, and STR **shall** be maintained in English under `docs/system/`. | Must | SYS-AS-006 | SUP | I | TEST-DOC-001 |
| SWR-DOC-061 | STD procedures **shall** reference `SWR-*` identifiers; STR **shall** record a result for each executed `TEST-*`. | Must | SYS-AS-006 | SUP | I | TEST-DOC-001 |
| SWR-DOC-062 | Interface changes that alter on-the-wire or shared-memory layout **shall** update ICD issue and affected SRS/STD traceability in the same baseline. | Must | SYS-AS-006 | SR | I | TEST-DOC-001 |
| SWR-DOC-063 | The suite **shall** state explicitly that no DAL/SIL/ASIL or airworthiness approval is claimed for this baseline, while retaining SG/HAZ identifiers for transfer. | Must | SYS-AS-006, CON-SAF-001 | SUP | I | TEST-DOC-001 |

---

## 11. Requirements index and parent traceability

| SWR ID | Title | Parent SYS |
|--------|-------|------------|
| SWR-ISO-001 | EPT private isolation | SYS-ISO-001 |
| SWR-ISO-002 | Shared page mapping | SYS-ISO-002 |
| SWR-ISO-003 | EPT violation handling | SYS-ISO-003 |
| SWR-ISO-004 | Isolation model overlap reject | SYS-ISO-004 |
| SWR-ISO-005 | Permission bits | SYS-ISO-004 |
| SWR-DP-010 | no_std guests | SYS-DP-001 |
| SWR-DP-011 | Bidirectional forward | SYS-DP-001 |
| SWR-DP-012 | Boot-info driven | SYS-AS-003 |
| SWR-DP-013 | Ring-full drop | SYS-DP-001 |
| SWR-DP-014 | Stats updates | SYS-DP-005 |
| SWR-DP-015 | SPSC ring semantics | SYS-DP-001 |
| SWR-NIC-020 | NicBackend API | SYS-DP-002 |
| SWR-NIC-021 | Sriov selection | SYS-DP-002 |
| SWR-NIC-022 | Inject selection | SYS-DP-003 |
| SWR-NIC-023 | Sriov fail-fast | SYS-DP-004 |
| SWR-NIC-024 | Config-only switch | SYS-DP-003 |
| SWR-CFG-030 | TOML load | SYS-AS-003 |
| SWR-CFG-031 | Incomplete reject | SYS-AS-003 |
| SWR-CFG-032 | Env overrides | SYS-AS-003 |
| SWR-CFG-033 | No hardcoding | SYS-AS-003 |
| SWR-CFG-034 | Example config | SYS-AS-003 |
| SWR-CFG-035 | QEMU from config | SYS-AS-003 |
| SWR-AS-040 | Verus specs | SYS-AS-001 |
| SWR-AS-041 | TCB list | SYS-AS-005 |
| SWR-AS-042 | Coverage ≥ 90% | SYS-AS-002 |
| SWR-AS-043 | Coverage allowlist | SYS-AS-002 |
| SWR-AS-044 | No business mocks | CON-QA-001 |
| SWR-HVN-050 | xtask build | SYS-AS-004 |
| SWR-HVN-051 | xtask run-nested | SYS-AS-004 |
| SWR-HVN-052 | xtask smoke | SYS-DP-001 |
| SWR-HVN-053 | xtask bench | SYS-DP-005 |
| SWR-HVN-054 | Bench vs target | SYS-DP-006 |
| SWR-HVN-055 | Prefer Sriov bench | SYS-DP-006 |
| SWR-HVN-056 | HARNESS.md | SYS-AS-004 |
| SWR-HVN-057 | verify + coverage cmds | SYS-AS-001/002 |
| SWR-HVN-058 | Bench report metadata | SYS-DP-007 |
| SWR-DOC-060 | English suite | SYS-AS-006 |
| SWR-DOC-061 | STD/STR trace | SYS-AS-006 |
| SWR-DOC-062 | ICD change CM | SYS-AS-006 |
| SWR-DOC-063 | Non-certification statement | CON-SAF-001 |

**Withdrawn IDs:** None.  
**N/A for demonstrator:** bare-metal boot software requirements — None defined.  
**Not claimed:** any DAL / SIL / ASIL assignment to `SWR-*`.

---

## 12. Change record

| Issue | Date | Description |
|-------|------|-------------|
| 0.1 | 2026-10-26 | Skeleton seed |
| 0.2 | 2026-10-26 | Renumbered area IDs; full shall attributes; parent/V&V/procedure columns |
| 0.3 | 2026-10-26 | Safety significance column; SG parents; SWR-HVN-058 / SWR-DOC-063; airborne rate honesty |
