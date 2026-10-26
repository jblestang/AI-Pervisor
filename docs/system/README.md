# System documentary suite

Home-grown documentary package for the **Hyperviseur edge gateway** spike (Type‑1 nested hypervisor, WAN/LAN guests, shared ring, SR‑IOV / Inject).

Language: **English**. Structure: intentional and compact — not an IEEE/ECSS template.

## Documents

| Doc | File | Role |
|-----|------|------|
| SSS | [SSS.md](./SSS.md) | System/Subsystem Specification — system needs and scope |
| SSDD | [SSDD.md](./SSDD.md) | System/Subsystem Design Description — architecture |
| SRS | [SRS.md](./SRS.md) | Software Requirements Specification — testable software requirements |
| ICD | [ICD.md](./ICD.md) | Interface Control Document — ABIs and config schema |
| STD | [STD.md](./STD.md) | Software Test Description — procedures and xtask mapping |
| STR | [STR.md](./STR.md) | Software Test Report — campaign results (filled after Task 11) |

Related design/plan (Project Context, not necessarily in this repo):

- Design: `docs/superpowers/specs/2026-10-26-hypervisor-spike-design.md`
- Plan: `docs/superpowers/plans/2026-10-26-hypervisor-spike.md`

Repo companions (created in later tasks): `docs/TCB.md`, `docs/HARNESS.md`, `config/demo.example.toml`.

## Requirement and interface ID scheme

| Prefix | Used in | Meaning |
|--------|---------|---------|
| `SYS-` | SSS | System / subsystem need |
| `SWR-` | SRS | Software requirement (testable) |
| `IF-` | ICD | Interface or schema element |
| `TEST-` | STD | Test procedure |

Rules:

1. Every `SWR-` has a measurable acceptance criterion.
2. Every `TEST-` references one or more `SWR-` (and optionally `SYS-` / `IF-`).
3. `STR` records pass/fail per `TEST-` after the campaign.
4. **ICD** is the source of truth for wire formats and config fields — not hardcoded literals in source.

## Traceability (spike)

```text
SYS-*  →  SWR-*  →  TEST-*  →  STR results
              ↑
           IF-* (ICD)
```

## Status

| Doc | Spike status |
|-----|----------------|
| SSS / SSDD / SRS / ICD / STD | Seeded in Task 0; refined as code lands |
| STR | Template only until smoke / bench / coverage / Verus complete (Task 11) |
