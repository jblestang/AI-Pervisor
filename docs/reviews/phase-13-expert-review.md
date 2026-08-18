# Phase 13 expert review

Multi-domain review of Gate C CPU instruction seams: host CPUID probes, validate-only instruction disposition, CPU seam backends, and host-tested Gate C CPU seam orchestration (`cursor/phase-13-cpu-instructions-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| CPUID probes | `hv-x86-cpu::cpuid` — VMX/EPT capability checks, VT-d host eligibility |
| CPU instruction seams | `run_*_cpu_seam()`, `CpuInstructionDisposition` (SeamValidated / Executed / SkippedNoHardware) |
| CPU seam backends | `CpuSeam*Backend` — structure programming + seam validation chain |
| Gate C CPU seam orchestration | `boot_*_gate_c_cpu_seam*()` with `GateCCpuSeamResult` (`cpu-seams` feature) |
| UEFI hypervisor entry | Unchanged — still mock Gate C from Phase 11 (no `hv-x86-cpu` in firmware chain) |
| Build / CI | Host-tested CPU seam path via `hv-hypervisor` default `cpu-seams`; OVMF smoke unchanged |

## Phase 12 deferrals closed

| Phase 12 item | Phase 13 disposition |
|---------------|---------------------|
| VMX/EPT/VT-d CPU instructions (Phase 12 #7) | **Partially closed** — CPU instruction seams validate capabilities and record disposition; default builds do not execute `vmxon`/EPT pointer/IOMMU enable. |
| UEFI hardware programming (Phase 12 #8) | **Unchanged** — firmware remains mock Gate C; CPU seams are host-only behind optional `cpu-seams` feature. |

## Code coverage

Fresh run: `cargo xtask coverage` (2026-08-18).

| Metric | Value |
|--------|-------|
| Workspace line coverage | **95.01%** (8364 lines, 417 missed) |
| Minimum threshold | 95% |
| Result | **pass** |

## Domain expert notes

### CPUID probes (`hv-x86-cpu::cpuid`)

- **Finding:** CPU seams must consult live CPU capabilities before recording instruction disposition.
- **Fix:** `cpuid_vmx_available()` / `cpuid_ept_available()` use `__cpuid_count`; VT-d eligibility is architecture-gated (platform validation owns DMAR presence).
- **Risk (deferred):** No `IA32_VMX_BASIC` revision read; reference revision from Phase 12 programming remains MODEL-only.

### CPU instruction seams (`hv-x86-cpu::seams`)

- **Finding:** Gate C needs a CI-safe seam between structure programming and live privileged instructions.
- **Fix:** `run_vmxon_cpu_seam()`, `run_ept_pointer_cpu_seam()`, and `run_vtd_enable_cpu_seam()` validate programmed artifacts, probe CPUID, and return `SeamValidated` by default. Optional `execute-instructions` feature records `Executed` without inline asm in Phase 13.
- **Risk (deferred):** No actual `vmxon`, `invept`, or IOMMU MMIO; live execution deferred to Phase 14+.

### CPU seam backends

- **Finding:** Host orchestration should chain Phase 12 programming with Phase 13 seams without duplicating init logic.
- **Fix:** `CpuSeamVmxBackend`, `CpuSeamEptBackend`, and `CpuSeamVtdBackend` implement existing backend traits by programming structures then running CPU seams.
- **Risk (deferred):** Seam failure mapping reuses planning/backend error kinds; future live-instruction errors may need dedicated kinds.

### Gate C CPU seam orchestration

- **Finding:** Host path needs orchestration distinct from programming-only and mock UEFI paths.
- **Fix:** `GateCCpuSeamResult` wraps `GateCProgrammingResult` plus optional seam outcomes; `boot_*_gate_c_cpu_seam*()` entry points added behind `cpu-seams` feature on `hv-hypervisor-boot`.
- **Risk (deferred):** Three Gate C host paths (mock, programming, CPU seam) share orchestration helpers; consolidation remains future work.

### UEFI / firmware path

- **Finding:** Firmware images must not pull host CPUID/unsafe instruction crates.
- **Disposition:** `hv-hypervisor-efi` continues Phase 11 mock Gate C; `cpu-seams` is enabled only on host `hv-hypervisor` re-exports.
- **Risk (deferred):** Firmware CPU seams require fixed buffers and a dedicated unsafe/firmware split.

## Findings and disposition

### MUST FIX (applied)

1. **`hv-x86-cpu` crate** — CPUID probes, instruction seams, CPU seam backends.
2. **Gate C CPU seam orchestration** — `GateCCpuSeamResult` and `boot_*_gate_c_cpu_seam*()` host entry points.
3. **Feature gating** — `cpu-seams` optional on `hv-hypervisor-boot`; default on host `hv-hypervisor` only.

### SHOULD FIX (applied)

4. **Coverage and tests** — CPU seam unit/integration tests; workspace line coverage **95.01%**.
5. **Documentation** — Architecture, platform contract, proof levels, README updated.

### Documented (deferred)

6. **Live privileged instructions** — No inline `vmxon`/EPT/VT-d MMIO even with `execute-instructions`.
7. **UEFI CPU seams** — Mock Gate C on firmware; CPU seams host-only.
8. **Production KVM OVMF hardware path** — Not exercised in CI.
9. **e1000 datapath (Gate D)** — Unchanged.

## Delivered

| Component | Role |
|-----------|------|
| `hv-x86-cpu` | CPUID + CPU instruction seams + `CpuSeam*Backend` |
| `hv-hypervisor-boot::gate_c` | `GateCCpuSeamResult`, `boot_*_gate_c_cpu_seam*` (`cpu-seams`) |
| `hv-hypervisor` | Host re-exports CPU seam entry points (default `cpu-seams`) |

## Verification

- `cargo test --workspace` — pass
- `cargo test -p hv-hypervisor-boot --features cpu-seams` — pass
- `cargo clippy --all-targets --all-features -- -D warnings` — pass
- `cargo xtask coverage` — pass (**95.01%** line coverage)
- `cargo xtask build-boot-chain` — pass
- `cargo xtask ovmf-smoke-boot` — pass

## Review status

All MUST and SHOULD items above are applied. Phase 12 CPU instruction deferral is partially closed (validate-only seams; live execution deferred). PR **#14** is ready for human review.
