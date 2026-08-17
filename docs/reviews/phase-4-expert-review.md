# Phase 4 expert review

Retrospective multi-domain review of Gate B foundation (`cursor/phase-4-gate-b-foundation-0b4f`, tip `911f2fb`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Observed platform model | `ObservedPlatform`, JSON fixtures, `parse_observed_platform_json` |
| Platform validation | `validate_platform` fail-closed requirement comparison |
| Static layout planning | `plan_static_platform_ir`, `StaticPlatformIR`, fixed `PLATFORM_PHYS_BASE` |
| Artifact integration | `hv-config generate` → `static-platform-layout.json`, `platform-layout.txt` |
| Config ↔ platform contract | `PlatformRequirements` vs observed features, RAM, page sizes, PCI presence |

## Findings and disposition

### Gate B foundation criteria (met at Phase 4)

1. **Observed platform schema** — `ObservedPlatform` captures CPU features, RAM, page sizes, SMT, and PCI BDF list with JSON fixtures (`qemu_reference.json`, `missing_vmx.json`).
2. **Fail-closed validation** — `validate_platform()` rejects arch mismatches, missing required features, insufficient RAM/cores/page sizes, and absent expected PCI devices; optional/preferred features surface warnings without silent downgrade.
3. **Deterministic layout planner** — `plan_static_platform_ir()` sorts partitions and IPC channels by ID, aligns regions to 4 KiB, assigns contiguous host physical addresses from `PLATFORM_PHYS_BASE`, and records PCI ownership from intent IR.
4. **Artifact pipeline** — `hv-config generate` emits static platform layout artifacts alongside existing Gate A review outputs.
5. **Host-side integration tests** — Reference fixture passes reference `configs/qemu.yaml` requirements; negative fixtures reject missing VMX and insufficient RAM.

### SHOULD FIX (closed in later phases)

6. **Runtime observation** — Phase 4 accepts fixture JSON only. Live CPUID, UEFI memory-map, and ACPI capability observation landed in Phase 5 (`observe_platform`, `CpuidSnapshot`); see [phase-5-expert-review.md](phase-5-expert-review.md).
7. **Firmware PCI discovery** — Phase 4 validates PCI BDF presence against requirements but does not enumerate config space. Loader firmware PCI scan and transfer forwarding landed in Phase 8; see [phase-8-expert-review.md](phase-8-expert-review.md).
8. **Observed platform JSON fuzzing** — `observed_platform_json` and `observe_platform` fuzz targets added with Phase 5+ parser hardening; see [fuzzing.md](../fuzzing.md).

### Documented (deferred at Phase 4)

9. **Fixed physical base** — `PLATFORM_PHYS_BASE` (`0x0000_0001_0000_0000`) is a compile-time constant; negotiation with firmware/E820 or MTRR constraints is deferred to VMX/EPT bring-up (Phase 9+).
10. **SMT exclusive-core policy** — When `SmtPolicy::ExclusiveCore` is configured and SMT is observed enabled, validation emits a warning rather than failing. Compile-time semantic checks still reject overlapping physical cores across partitions under that policy; runtime SMT disablement proof remains future work.
11. **PCI layout beyond ownership** — Planner records BDF → VM assignment only; BAR decoding, MMIO windows, and VT-d grouping are not planned at Phase 4 (Gate C+).
12. **QEMU / real-hardware proof** — Observed platform proof levels remain `QEMU + REAL_HW (future)` in [proof-levels.md](../proof-levels.md); Phase 4 proof is host fixture + unit tests only.
13. **JSON bomb bounds** — `parse_observed_platform_json` trusts serde without explicit vec length caps; acceptable for operator-supplied fixtures at Phase 4; hostile input hardening relies on fuzzing and host-only consumption until firmware paths consume observation blobs (Phase 5+ bounded parsers).

## Delivered

| Component | Role |
|-----------|------|
| `hv-platform-model::observed` | Observed platform snapshot type and JSON parse |
| `hv-platform-model::validate` | Requirements vs observation comparison with warnings |
| `hv-platform-model::planner` | Deterministic `StaticPlatformIR` from `StaticIntentIR` |
| `hv-platform-model::platform_ir` | Planned guest, IPC, reserve, and PCI ownership regions |
| `tests/fixtures/observed/` | Reference and negative validation fixtures |
| `hv-config generate` | Writes `static-platform-layout.json` and human-readable layout |

## Verification

Reviewed at current workspace tip (includes later-phase fixes; Phase 4 validation/planner behavior unchanged):

- `cargo test -p hv-platform-model` — pass
- `cargo test -p hv-config-model -p hv-platform-model` — pass
- `cargo clippy -p hv-platform-model --all-targets -- -D warnings` — pass
- `cargo xtask config validate configs/qemu.yaml` — pass
- `cargo xtask coverage` — pass (≥ 95% workspace line coverage)

## Review status

Gate B foundation criteria are met at Phase 4. Runtime observation and firmware PCI discovery deferrals (6–7) are closed in Phases 5 and 8. Physical layout, BAR planning, and silicon proof items (9–12) remain explicitly deferred with phase ownership.
