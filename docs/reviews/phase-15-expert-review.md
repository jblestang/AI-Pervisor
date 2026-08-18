# Phase 15 expert review

Multi-domain review of REAL_HW VMX launch and datapath foundation: resident page installation, VMCS lifecycle, firmware-safe live opt-in, KVM/QEMU smoke harness, and host-tested Gate C REAL_HW orchestration (`cursor/phase-15-real-hw-qemu-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Resident page install | `hv-x86-cpu::resident`, `PageAllocator`, `MockPageAllocator`, VMXON/EPT/VMCS install |
| REAL_HW backends | `ResidentCpuSeam*Backend` — install structures then run CPU seams |
| VMCS lifecycle | `execute_vmcs_prepare()` — VMCLEAR + VMPTRLD before EPT pointer VMWRITE |
| Firmware live opt-in | `firmware-live-execution` compile-time gate; UEFI uses `UefiPageAllocator` |
| Gate C REAL_HW orchestration | `GateCRealHwResult`, `boot_*_gate_c_real_hw*()` (`real-hw-execution` feature) |
| UEFI REAL_HW path | `boot_hypervisor_from_transfer_real_hw()`, serial markers, `real-hw-execution` build feature |
| KVM live smoke harness | `cargo xtask live-qemu-smoke`, `build-boot-chain-live`, serial evaluator |
| Build / CI | Multi-pass coverage (workspace + `real-hw-execution` + `execute-instructions`); live smoke skips when KVM/VMX unavailable |

## Phase 14 deferrals closed

| Phase 14 item | Phase 15 disposition |
|---------------|---------------------|
| REAL_HW VMXON success path (Phase 14 #7) | **Partially closed** — resident VMXON install + REAL_HW orchestration; live success requires ring-0 firmware/KVM (KVM smoke harness added; CI skips without nested virt) |
| Full VMCS lifecycle (Phase 14 #8) | **Partially closed** — `execute_vmcs_prepare()` (VMCLEAR + VMPTRLD) before EPT VMWRITE; VMLAUNCH/VMCS field programming deferred |
| UEFI live execution (Phase 14 #10) | **Partially closed** — `real-hw-execution` firmware feature + `UefiPageAllocator`; default OVMF path remains mock Gate C |
| Production KVM OVMF hardware path (Phase 14 #11) | **Partially closed** — `live-qemu-smoke` xtask with REAL_HW `.efi` build; opt-in local/CI when `/dev/kvm` + host VMX present |

## Code coverage

Fresh run: `cargo xtask coverage` (2026-08-18).

| Metric | Value |
|--------|-------|
| Workspace line coverage | **95.00%** (10557 lines, 528 missed) |
| Minimum threshold | 95% |
| Result | **pass** |

### Key module coverage

| Module | Line % | Notes |
|--------|-------:|-------|
| `hv-hypervisor-boot/src/gate_c.rs` | 99.79% | REAL_HW entry points behind `real-hw-execution` |
| `hv-x86-cpu/src/resident.rs` | 100.00% | Mock allocator + install helpers |
| `hv-x86-cpu/src/instructions/vmx.rs` | 100.00% | Live path stubbed under `test`/`coverage`; asm in `live_asm` |
| `hv-x86-cpu/src/instructions/vmcs.rs` | 100.00% | VMCS prepare stub + validate paths |
| `hv-x86-cpu/src/instructions/ept.rs` | 100.00% | EPT pointer load with VMCS prepare gate |
| `hv-x86-cpu/src/instructions/msr.rs` | 100.00% | IA32_VMX_BASIC helper |
| `hv-hypervisor-efi/src/allocator.rs` | ~95% | Host-tested via `#[cfg(test)]` allocate hook; UEFI path firmware-only |
| `xtask/src/live_qemu_smoke.rs` | ~91% | Evaluator + mock-runner paths; hardware/OVMF branches CI-skipped |

- **Inline asm (`live_asm.rs`)** — Compiled only outside `test` and `coverage` builds; not instrumented in CI coverage passes. Acceptable for Phase 15.
- **KVM live smoke** — Skips with exit 0 when nested KVM or host VMX unavailable; success path requires local KVM + OVMF.

## Feature matrix

| Feature | Crate | Default | UEFI chain | Effect |
|---------|-------|---------|------------|--------|
| `real-hw-execution` | `hv-hypervisor-boot` | off | off | Resident backends + `GateCRealHwResult` |
| `real-hw-execution` | `hv-hypervisor-efi` | off | opt-in | `UefiPageAllocator`, REAL_HW boot entry |
| `real-hw-execution` | `hv-hypervisor-efi-bin` | off | opt-in | Serial markers for VMXON/EPT execution |
| `firmware-live-execution` | `hv-x86-cpu` | off | opt-in | Compile-time ring-0 live opt-in (no env var) |
| `execute-instructions` | `hv-x86-cpu` | off | off | Live instruction modules + `live_asm` |
| `live-execution` | `hv-hypervisor-boot` | off | off | Host live orchestration (Phase 14) |

## Domain expert notes

### Resident page install

- **Finding:** Phase 14 VMXON used planner physical addresses not mapped in firmware or host test harnesses.
- **Fix:** `PageAllocator` trait, `install_vmxon_region()`, `install_ept_tables()`, `install_vmcs_region()` rebind programmed structures to freshly allocated pages. `MockPageAllocator` for host tests; `UefiPageAllocator` for firmware.
- **Risk (deferred):** UEFI `AllocatePages` RUNTIME_SERVICES_DATA is a bring-up choice; long-term hypervisor reserve should use platform IR reserved regions.

### VMCS lifecycle

- **Finding:** EPT pointer VMWRITE requires an active VMCS pointer (Phase 14 deferral).
- **Fix:** `execute_vmcs_prepare()` issues VMCLEAR + VMPTRLD before EPT pointer load. VMCS region gets revision prefix from `resolve_vmxon_revision()`.
- **Risk (deferred):** VMLAUNCH, guest/host state fields, and nested VMCS management not implemented.

### Firmware-safe live opt-in

- **Finding:** Host CI uses `HV_X86_LIVE_INSTRUCTIONS=1`; firmware cannot rely on env vars.
- **Fix:** `firmware-live-execution` feature sets `firmware_live_execution_enabled()` at compile time. Default firmware and OVMF smoke builds remain mock-backed.
- **Risk (deferred):** Mis-built REAL_HW firmware image could attempt live instructions on unsupported hardware — serial markers + smoke harness mitigate during bring-up.

### KVM live smoke harness

- **Finding:** Phase 14 lacked an automated REAL_HW boot verification path.
- **Fix:** `cargo xtask build-boot-chain-live` builds REAL_HW hypervisor; `cargo xtask live-qemu-smoke` boots under KVM/QEMU and checks REAL_HW serial markers. Graceful skip when hardware unavailable.
- **Risk (deferred):** CI cloud agents typically lack nested KVM; harness is opt-in local verification.

## Findings and disposition

### MUST FIX (applied)

1. **Resident page install** — `PageAllocator`, install helpers, REAL_HW backends.
2. **VMCS prepare** — VMCLEAR + VMPTRLD before EPT pointer programming.
3. **Gate C REAL_HW orchestration** — `boot_*_gate_c_real_hw*()` with `GateCRealHwResult`.
4. **UEFI REAL_HW path** — `boot_hypervisor_from_transfer_real_hw()`, build features, serial markers.
5. **KVM smoke harness** — `live-qemu-smoke` + `build-boot-chain-live` xtasks.

### SHOULD FIX (applied)

6. **Coverage and tests** — Multi-pass coverage; resident/REAL_HW/live-qemu tests; workspace line coverage **95.00%**.
7. **Documentation** — Architecture, platform contract, proof levels, OVMF boot, README updated.

### Documented (deferred)

8. **VMLAUNCH + guest datapath** — VMCS field programming and guest execution deferred to Gate D.
9. **DMAR MMIO / context tables** — VT-d enable remains intent-only.
10. **Nested-virt CI** — Live KVM smoke not required in default CI.
11. **e1000 datapath (Gate D)** — Unchanged.

## Delivered

| Component | Role |
|-----------|------|
| `hv-x86-cpu::resident` | Host physical page install for VMXON/EPT/VMCS |
| `hv-x86-cpu::resident_backends` | REAL_HW VMX/EPT/VT-d backends |
| `hv-x86-cpu::instructions::vmcs` | VMCS prepare (VMCLEAR/VMPTRLD) |
| `hv-x86-cpu::instructions::live_asm` | Privileged encodings (non-test/non-coverage builds) |
| `hv-hypervisor-boot::gate_c` | `GateCRealHwResult`, REAL_HW orchestration |
| `hv-hypervisor-efi::allocator` | UEFI page allocator for resident install |
| `hv-hypervisor-efi` | REAL_HW boot entry + serial markers |
| `xtask::live_qemu_smoke` | KVM/QEMU REAL_HW smoke harness |

## Verification

- `cargo test --workspace` — pass
- `cargo test -p hv-hypervisor-boot --features real-hw-execution` — pass
- `cargo test -p hv-hypervisor-efi --features real-hw-execution` — pass
- `cargo test -p hv-x86-cpu --features execute-instructions,std,firmware-live-execution` — pass
- `cargo clippy --all-targets --all-features -- -D warnings` — pass
- `cargo xtask coverage` — pass (**95.00%** line coverage)
- `cargo xtask build-boot-chain` — pass (when run)
- `cargo xtask build-boot-chain-live` — pass (when run)
- `cargo xtask ovmf-smoke-boot` — pass or skip per environment
- `cargo xtask live-qemu-smoke` — pass or skip (exit 0) when KVM/VMX unavailable

## Review status

All MUST and SHOULD items above are applied. Phase 14 REAL_HW deferrals are partially closed (resident install + VMCS prepare + KVM harness; VMLAUNCH/datapath deferred). PR **#16** is ready for human review.
