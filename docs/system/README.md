# Hypervisor Edge Gateway — System Documentary Suite

| Field | Value |
|-------|-------|
| **Suite ID** | HV-EDGE-DOC-SUITE |
| **Programme** | AI-Pervisor / Hyperviseur edge gateway technology demonstrator |
| **Issue** | 0.2 |
| **Date** | 2026-10-26 |
| **Language** | English |
| **Classification** | UNCLASSIFIED — Technology Demonstrator |
| **Structure** | Home-grown systems-engineering package (not claimed IEEE/ECSS conformity) |

---

## 1. Purpose of this suite

This package is the **authoritative documentary baseline** for the Hypervisor Edge Gateway technology demonstrator (Type‑1 nested hypervisor, dual WAN/LAN guests, shared-memory datapath, SR‑IOV or Inject host I/O).

It is intended for review by programme, systems, software, hardware, and independent test stakeholders — including governmental or institutional technical authorities — as evidence that:

1. System needs are stated, constrained, and success-gated.  
2. Design allocates functions to hardware, firmware/hypervisor, and guest software.  
3. Software requirements are uniquely identified, verifiable, and traced.  
4. External and internal interfaces are controlled (ICD is normative for layouts and policy).  
5. Verification & validation (V&V) is planned with executable procedures and evidence artefacts.  
6. Test results (when available) close the loop from need → requirement → procedure → result.

**Maturity statement.** This issue documents a **technology demonstrator** (spike) under nested QEMU/KVM. It is not a product certification data package. Claims are limited to the stated scope and verification methods. Where implementation is incomplete, the suite records **applicability** and **verification status** rather than silent omission.

## 2. Document set

| Doc ID | Short | File | Primary audience | Role |
|--------|-------|------|------------------|------|
| HV-EDGE-SSS-001 | SSS | [SSS.md](./SSS.md) | System eng., programme | System / subsystem specification — mission need, context, SYS requirements |
| HV-EDGE-SSDD-001 | SSDD | [SSDD.md](./SSDD.md) | System / SW / HW eng. | Design description — architecture, allocation, modes, TCB |
| HV-EDGE-SRS-001 | SRS | [SRS.md](./SRS.md) | Software eng., V&V | Software requirements — shall statements, attributes, V&V method |
| HV-EDGE-ICD-001 | ICD | [ICD.md](./ICD.md) | SW/HW interface owners | Interface control — ring, NIC, boot blob, UART, stats, config |
| HV-EDGE-STD-001 | STD | [STD.md](./STD.md) | Test eng., V&V | Software test description — procedures, environments, evidence |
| HV-EDGE-STR-001 | STR | [STR.md](./STR.md) | Test eng., authority | Software test report — campaign results and sign-off (filled post-campaign) |

**Companion artefacts** (repository, outside this folder):

| Artefact | Role |
|----------|------|
| `docs/TCB.md` | Trusted Computing Base — enumerated `unsafe` (when code baseline exists) |
| `docs/HARNESS.md` | Laboratory harness / VFIO / nested SR‑IOV procedure |
| `docs/VERUS.md` | Verus toolchain alignment and proof status |
| `config/demo.example.toml` | Normative example of `IF-CFG-*` complete configuration |

## 3. Identification scheme

| Prefix | Document | Semantics | Example |
|--------|----------|-----------|---------|
| `SYS-` | SSS | System / subsystem requirement | `SYS-ISO-001` |
| `SWR-` | SRS | Software requirement | `SWR-DP-011` |
| `IF-` | ICD | Interface or schema element | `IF-RING-001` |
| `TEST-` | STD | Test procedure | `TEST-SMOKE-001` |
| `ASM-` / `CON-` / `RSK-` | SSS/SSDD | Assumption / constraint / risk | `ASM-LAB-001` |

**Rules (normative for this suite):**

1. Every `SWR-*` shall have a verification method (Test, Demonstration, Analysis, or Inspection) and at least one planned `TEST-*` or inspection checklist item.  
2. Every `TEST-*` shall reference the `SWR-*` (and optionally `SYS-*` / `IF-*`) it verifies.  
3. `STR` shall record result per `TEST-*` for the declared campaign.  
4. **ICD prevails** over source literals for interface layouts and configuration field meaning; source shall implement ICD.  
5. Identifiers are stable across issues; retired IDs are marked **Withdrawn**, never reused for a different meaning.

## 4. Traceability model

```text
  Mission / programme need
            │
            ▼
        SYS-*  (SSS)  ──constraints/assumptions/risks──► SSDD allocation
            │
            ▼
        SWR-*  (SRS)  ◄──── IF-* (ICD) ────► implementation crates
            │
            ▼
       TEST-*  (STD)  ──evidence artefacts──► STR results + authority review
```

## 5. Configuration management

| Item | Practice |
|------|----------|
| Storage | Version-controlled in git (`docs/system/`) |
| Baseline | Suite Issue + git commit SHA recorded in STR campaign metadata |
| Change control | Material requirement or interface changes → Issue bump + change record in affected docs |
| Language | English only for this suite |

## 6. Document control — suite change record

| Issue | Date | Description | Author role |
|-------|------|-------------|-------------|
| 0.1 | 2026-10-26 | Initial documentary skeletons (Task 0) | Engineering |
| 0.2 | 2026-10-26 | Government-review quality upgrade: document control, CONOPS, shall language, HW/SW allocation, normative ICD, V&V campaign structure | Systems / SW / HW / Test eng. |

## 7. Review and approval (template)

| Role | Name | Organisation | Date | Signature |
|------|------|--------------|------|-----------|
| Systems engineering | | | | |
| Software engineering | | | | |
| Hardware / platform engineering | | | | |
| Independent test / V&V | | | | |
| Programme / technical authority | | | | |

*Signature blocks are completed when this suite is formally baselined for external submission.*

## 8. Reading order for external reviewers

1. This index (scope and ID rules)  
2. SSS — what the system must achieve and what is out of scope  
3. SSDD — how it is designed and allocated  
4. SRS — software shalls  
5. ICD — interface contracts  
6. STD — how compliance is demonstrated  
7. STR — what was actually measured (when campaign complete)  
