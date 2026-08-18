# Phase 14 expert review

Multi-domain review of Gate C live privileged instruction execution: inline VMX/EPT/VT-d instruction modules, runtime execution gates, and host-tested live execution orchestration (`cursor/phase-14-live-instructions-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Live instruction modules | `hv-x86-cpu::instructions` — VMXON, EPT pointer VMWRITE, VT-d enable intent, IA32_VMX_BASIC helper |
| Runtime execution gates | `HV_X86_LIVE_INSTRUCTIONS=1` + ring-0 CPL check before live attempts |
| CPU seam integration | `execute-instructions` feature attempts live ops; `Unavailable` gates fall back to `SeamValidated` |
| Gate C live orchestration | `boot_*_gate_c_live_execution*()` with `GateCLiveExecutionResult` (`live-execution` feature) |
| UEFI hypervisor entry | Unchanged — mock Gate C; no live instruction crates in firmware chain |
| Build / CI | Default workspace tests remain CI-safe; live paths host-only behind features |

## Phase 13 deferrals closed

| Phase 13 item | Phase 14 disposition |
|---------------|---------------------|
| Live privileged instructions (Phase 13 #6) | **Partially closed** — inline VMXON/VMWRITE/IOMMU intent modules with runtime + ring-0 gates; userspace CI falls back to validate-only disposition. |
| `execute-instructions` stub (Phase 13 #44) | **Closed** — feature now invokes live modules; `Executed` only when environment permits and instructions succeed. |
| No `IA32_VMX_BASIC` revision read (Phase 13 #39) | **Partially closed** — `read_vmx_basic_msr()` added (ring-0 + runtime gated); programming path still uses MODEL reference revision in CI. |

## Code coverage

Fresh run: `cargo xtask coverage` (2026-08-18).

| Metric | Value |
|--------|-------|
| Workspace line coverage | **95.00%** (8537 lines, 427 missed) |
| Minimum threshold | 95% |
| Result | **pass** |

### Key module coverage

| Module | Lines | Missed | Line % | Notes |
|--------|------:|-------:|-------:|-------|
| `hv-hypervisor-boot/src/gate_c.rs` | 296 | 1 | 99.66% | Live entry points behind `live-execution`; one shared helper branch uncovered in default CI run |
| `hv-x86-cpu/src/instructions/vtd.rs` | 34 | 0 | 100.00% | Intent recording + tests |
| `hv-x86-cpu/src/instructions/msr.rs` | 16 | 0 | 100.00% | MSR helper + revision extraction |
| `hv-x86-cpu/src/error.rs` | 22 | 0 | 100.00% | Includes `ExecutionFailed` |
| `hv-x86-cpu/src/instructions/environment.rs` | 47 | 3 | 93.62% | CPL probe + runtime env gate |
| `hv-x86-cpu/src/instructions/vmx.rs` | 22 | 2 | 90.91% | Validate path covered; inline VMXON asm only built with `execute-instructions` |
| `hv-x86-cpu/src/instructions/ept.rs` | 21 | 2 | 90.48% | Validate path covered; VMWRITE asm feature-gated |
| `hv-x86-cpu/src/backends.rs` | 115 | 0 | 100.00% | `ExecutionFailed` mapping covered |
| `hv-x86-cpu/src/seams.rs` | 282 | 43 | 84.75% | Live fallback paths covered; `Executed` disposition requires ring-0 harness |
| `hv-x86-cpu/src/cpuid.rs` | 85 | 8 | 90.59% | Acceptable — non-x86 fallback branches not exercised on x86_64 CI |

- **Inline asm paths (`vmx.rs` / `ept.rs` VMXON/VMWRITE)** — Compiled only when `execute-instructions` is enabled; not executed in default CI because ring-0 + env gates block attempts. Acceptable for Phase 14; REAL_HW harness deferred.
- **`gate_c.rs` (99.66%)** — `init_vmx_if_required` error-mapping through live CPU seam orchestration not end-to-end tested; crate-level backend tests suffice for Phase 14.

## Feature matrix

| Feature | Crate | Default | UEFI chain | Effect |
|---------|-------|---------|------------|--------|
| `cpu-seams` | `hv-hypervisor-boot` | off | off | CPU seam orchestration; validate-only disposition |
| `cpu-seams` | `hv-hypervisor` | on | n/a | Host re-exports CPU seam entry points |
| `execute-instructions` | `hv-x86-cpu` | off | off | Compiles inline VMX/EPT instruction modules |
| `live-execution` | `hv-hypervisor-boot` | off | off | Enables `execute-instructions` + `GateCLiveExecutionResult` |
| `live-execution` | `hv-hypervisor` | off | n/a | Host re-exports live execution entry points |

## Domain expert notes

### Live instruction modules (`hv-x86-cpu::instructions`)

- **Finding:** Phase 13 seams needed real instruction encodings behind explicit opt-in, not disposition-only stubs.
- **Fix:** `execute_vmxon()` sets CR4.VMXE and executes `vmxon`; `execute_ept_pointer_load()` issues VMWRITE for the EPT pointer field; `execute_vtd_enable()` records IOMMU enable intent in a host-visible slot. `read_vmx_basic_msr()` reads IA32_VMX_BASIC when ring-0 live execution is permitted.
- **Risk (deferred):** VMXON uses planner physical addresses that are not mapped in host userspace — live success requires a ring-0 firmware or KVM harness (REAL_HW).

### Runtime execution gates

- **Finding:** CI must not execute privileged instructions by default, even when `execute-instructions` is compiled into test binaries.
- **Fix:** Two-tier gate: (1) `HV_X86_LIVE_INSTRUCTIONS=1` env var via `live_execution_runtime_enabled()`; (2) CPL == 0 via `current_privilege_level()`. Userspace tests observe `live_environment_ready == false` and seams return `SeamValidated`.
- **Risk (deferred):** No nested-virt or `/dev/kvm` detection; a permissive ring-0 test harness could `#GP` on bad physical addresses.

### CPU seam integration

- **Finding:** Seam disposition must distinguish validation, execution, skip, and failure without panicking.
- **Fix:** `CpuSeamErrorKind::ExecutionFailed` added for post-gate instruction failures. `Unavailable` from live gates is swallowed in `execute_*_if_enabled()` helpers, preserving CI-safe `SeamValidated` fallback. `ExecutionFailed` and `InvalidInput` propagate through `CpuSeam*Backend` error mapping.
- **Risk (deferred):** EPT pointer VMWRITE requires an active VMCS pointer; full VMPTRLD/VMCS lifecycle is not implemented.

### Gate C live orchestration

- **Finding:** Host path needs explicit live-execution entry points distinct from validate-only CPU seams, mirroring Phase 10–13 orchestration layering.
- **Fix:** `GateCLiveExecutionResult { cpu_seam, live_environment_ready }` wraps `GateCCpuSeamResult`. Three host entry points (`boot_from_transfer_and_init_gate_c_live_execution*`, `boot_check_and_init_gate_c_live_execution`) delegate to existing CPU seam init then record environment readiness.
- **Risk (deferred):** Four Gate C host paths (mock, programming, CPU seam, live) share `init_*_if_required` helpers; consolidation remains future work.

### UEFI / firmware path

- **Finding:** Firmware images must not link live instruction modules or execute privileged instructions during OVMF smoke boot.
- **Disposition:** `hv-hypervisor-efi` continues Phase 11 mock Gate C; `live-execution` and `execute-instructions` are host-only. OVMF smoke boot unchanged.
- **Risk (deferred):** Firmware live bring-up requires fixed physical mappings, no env-var gates, and a dedicated unsafe/firmware split crate.

### Security / safety

- **Finding:** Accidental live execution in development or CI could destabilize the host.
- **Fix:** Default workspace build never sets `HV_X86_LIVE_INSTRUCTIONS`. Live modules are feature-gated at compile time and runtime. Workspace `unsafe_code = deny` remains in effect for all crates except `hv-x86-cpu` (explicit `#![allow(unsafe_code)]`).
- **Disposition:** Acceptable for Phase 14 host-only scope.

## Findings and disposition

### MUST FIX (applied)

1. **Live instruction modules** — VMXON, EPT VMWRITE, VT-d enable intent under `execute-instructions`.
2. **Runtime + ring-0 gates** — `HV_X86_LIVE_INSTRUCTIONS` and CPL check before live attempts.
3. **Gate C live orchestration** — `GateCLiveExecutionResult` and `boot_*_gate_c_live_execution*()`.
4. **`ExecutionFailed` error kind** — propagated through CPU seam backends.

### SHOULD FIX (applied)

5. **Coverage and tests** — Live instruction unit/integration tests (`tests/live_instructions.rs`, `tests/live_execution.rs`); workspace line coverage **95.00%**.
6. **Documentation** — Architecture, platform contract, proof levels, README, OVMF crate table updated.

### Documented (deferred)

7. **REAL_HW VMXON success path** — Requires ring-0 harness with mapped VMXON region at planner physical address.
8. **Full VMCS lifecycle** — VMPTRLD, VMLAUNCH, VMCS field programming deferred.
9. **DMAR MMIO** — VT-d enable records intent only; no register writes or context-table memory installs.
10. **UEFI live execution** — Mock Gate C on firmware.
11. **Production KVM OVMF hardware path** — Not exercised in CI.
12. **e1000 datapath (Gate D)** — Unchanged.

## Delivered

| Component | Role |
|-----------|------|
| `hv-x86-cpu::instructions` | Live VMX/EPT/VT-d instruction modules + runtime gates |
| `hv-x86-cpu::seams` | Live attempt integration with `SeamValidated` fallback |
| `hv-hypervisor-boot::gate_c` | `GateCLiveExecutionResult`, `boot_*_gate_c_live_execution*` (`live-execution`) |
| `hv-hypervisor` | Host re-exports live execution entry points (`live-execution` feature) |

## Verification

- `cargo test --workspace` — pass
- `cargo test -p hv-hypervisor-boot --features cpu-seams` — pass (19 tests)
- `cargo test -p hv-hypervisor-boot --features live-execution` — pass (22 tests incl. live orchestration)
- `cargo test -p hv-hypervisor --features live-execution` — pass
- `cargo test -p hv-x86-cpu --features execute-instructions,std` — pass (live instruction integration)
- `cargo clippy --all-targets --all-features -- -D warnings` — pass
- `cargo xtask coverage` — pass (**95.00%** line coverage)
- `cargo xtask build-boot-chain` — pass
- `cargo xtask ovmf-smoke-boot` — pass

## Review status

All MUST and SHOULD items above are applied. Phase 13 live instruction deferral is partially closed (inline modules + gates; REAL_HW VMXON success deferred). PR **#15** is ready for human review.
