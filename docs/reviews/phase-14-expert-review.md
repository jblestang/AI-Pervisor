# Phase 14 expert review

Multi-domain review of Gate C live privileged instruction execution: inline VMX/EPT/VT-d instruction modules, runtime execution gates, and host-tested live execution orchestration (`cursor/phase-14-live-instructions-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Live instruction modules | `hv-x86-cpu::instructions` — VMXON, EPT pointer VMWRITE, VT-d enable intent |
| Runtime execution gates | `HV_X86_LIVE_INSTRUCTIONS=1` + ring-0 CPL check before live attempts |
| CPU seam integration | `execute-instructions` feature attempts live ops; falls back to `SeamValidated` when gated |
| Gate C live orchestration | `boot_*_gate_c_live_execution*()` with `GateCLiveExecutionResult` (`live-execution` feature) |
| UEFI hypervisor entry | Unchanged — mock Gate C; no live instruction crates in firmware chain |
| Build / CI | Default workspace tests remain CI-safe; live paths host-only behind features |

## Phase 13 deferrals closed

| Phase 13 item | Phase 14 disposition |
|---------------|---------------------|
| Live privileged instructions (Phase 13 #6) | **Partially closed** — inline VMXON/VMWRITE/IOMMU intent modules with runtime + ring-0 gates; userspace CI falls back to validate-only disposition. |
| `execute-instructions` stub (Phase 13 #44) | **Closed** — feature now invokes live modules; `Executed` only when environment permits and instructions succeed. |

## Code coverage

Fresh run: `cargo xtask coverage` (2026-08-18).

| Metric | Value |
|--------|-------|
| Workspace line coverage | **95.00%** (8537 lines, 427 missed) |
| Minimum threshold | 95% |
| Result | **pass** |

## Domain expert notes

### Live instruction modules (`hv-x86-cpu::instructions`)

- **Finding:** Phase 13 seams needed real instruction encodings behind explicit opt-in.
- **Fix:** `execute_vmxon()` (CR4.VMXE + VMXON), `execute_ept_pointer_load()` (VMWRITE EPT pointer), `execute_vtd_enable()` (records IOMMU enable intent). `read_vmx_basic_msr()` gated to ring 0.
- **Risk (deferred):** VMXON uses planner physical addresses; host userspace cannot map them — live success requires ring-0 firmware/KVM harness (REAL_HW).

### Runtime execution gates

- **Finding:** CI must not execute privileged instructions by default even with `execute-instructions` compiled.
- **Fix:** `live_execution_runtime_enabled()` requires `HV_X86_LIVE_INSTRUCTIONS=1`; `live_execution_environment_ready()` additionally requires CPL 0.
- **Risk (deferred):** No nested-virt detection; failed VMXON returns `ExecutionFailed` when attempted in permissive environments.

### CPU seam integration

- **Finding:** Seam disposition must distinguish validation, execution, skip, and failure.
- **Fix:** `CpuSeamErrorKind::ExecutionFailed` added; `Unavailable` from live gates maps to `SeamValidated` fallback in seams.
- **Risk (deferred):** EPT pointer load requires active VMCS; full VMCS lifecycle remains future work.

### Gate C live orchestration

- **Finding:** Host path needs explicit live-execution entry points distinct from validate-only CPU seams.
- **Fix:** `GateCLiveExecutionResult` wraps `GateCCpuSeamResult` plus `live_environment_ready`; `live-execution` feature enables `execute-instructions` on `hv-x86-cpu`.
- **Risk (deferred):** Four Gate C host paths (mock, programming, CPU seam, live) share helpers; consolidation deferred.

### UEFI / firmware path

- **Finding:** Firmware must not link live instruction modules.
- **Disposition:** UEFI hypervisor unchanged; `live-execution` host-only on `hv-hypervisor-boot`.
- **Risk (deferred):** Firmware live bring-up requires fixed physical mappings and dedicated unsafe crate split.

## Findings and disposition

### MUST FIX (applied)

1. **Live instruction modules** — VMXON, EPT VMWRITE, VT-d enable intent under `execute-instructions`.
2. **Runtime + ring-0 gates** — `HV_X86_LIVE_INSTRUCTIONS` and CPL check before live attempts.
3. **Gate C live orchestration** — `GateCLiveExecutionResult` and `boot_*_gate_c_live_execution*()`.
4. **`ExecutionFailed` error kind** — propagated through CPU seam backends.

### SHOULD FIX (applied)

5. **Coverage and tests** — Live instruction unit/integration tests; workspace line coverage **95.00%**.
6. **Documentation** — Architecture, platform contract, proof levels, README updated.

### Documented (deferred)

7. **REAL_HW VMXON success path** — Requires ring-0 harness with mapped VMXON region.
8. **Full VMCS lifecycle** — VMPTRLD/launch deferred.
9. **DMAR MMIO** — VT-d enable records intent only; no register writes.
10. **UEFI live execution** — Mock Gate C on firmware.
11. **e1000 datapath (Gate D)** — Unchanged.

## Delivered

| Component | Role |
|-----------|------|
| `hv-x86-cpu::instructions` | Live VMX/EPT/VT-d instruction modules + runtime gates |
| `hv-hypervisor-boot::gate_c` | `GateCLiveExecutionResult`, `boot_*_gate_c_live_execution*` (`live-execution`) |
| `hv-hypervisor` | Host re-exports live execution entry points (`live-execution` feature) |

## Verification

- `cargo test --workspace` — pass
- `cargo test -p hv-hypervisor-boot --features live-execution` — pass
- `cargo test -p hv-hypervisor --features live-execution` — pass
- `cargo clippy --all-targets --all-features -- -D warnings` — pass
- `cargo xtask coverage` — pass (**95.00%** line coverage)
- `cargo xtask build-boot-chain` — pass
- `cargo xtask ovmf-smoke-boot` — pass

## Review status

All MUST and SHOULD items above are applied. Phase 13 live instruction deferral is partially closed (inline modules + gates; REAL_HW success deferred). PR **#15** is ready for human review.
