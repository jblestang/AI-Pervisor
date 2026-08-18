# Phase 16 expert review

Multi-domain review of VMX launch and guest datapath foundation: VMCS field programming, VMLAUNCH instruction seam, smoke guest install, Gate C `vmx-launch` orchestration, and KVM smoke harness extensions (`cursor/phase-16-vmx-launch-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| VMCS field planning | `hv-vmx::launch`, `program_vmcs_fields`, `patch_guest_entry_in_fields` |
| VMLAUNCH instruction | `execute_vmlaunch()`, `live_asm::vmlaunch()`, `run_vmx_launch_cpu_seam()` |
| Guest boot foundation | `hv-guest-boot` boot info builder + `GUEST_SMOKE_IMAGE` |
| Resident guest install | `install_guest_image()` |
| Gate C vmx-launch orchestration | `GateCVmxLaunchResult`, `boot_*_gate_c_vmx_launch*()` |
| UEFI vmx-launch path | `boot_hypervisor_from_transfer_vmx_launch()`, serial markers |
| KVM launch smoke | Extended `live-qemu-smoke` marker parsing |
| Build / CI | Multi-pass coverage (`vmx-launch` feature passes) |

## Phase 15 deferrals closed

| Phase 15 item | Phase 16 disposition |
|---------------|---------------------|
| Full VMCS lifecycle (#8) | **Closed** — VMCS field programming + VMLAUNCH seam |
| VMLAUNCH + guest datapath (#8 deferred) | **Partially closed** — single-partition smoke guest + boot info; full IPC/NIC datapath deferred |
| REAL_HW VMX launch under KVM/OVMF | **Closed** when nested virt available (`vmx-launch` EFI build) |
| Nested VMCS / multi-vCPU launch | **Deferred** |
| e1000 datapath (Gate D) | **Unchanged** |
| DMAR MMIO | **Unchanged** |

## Feature matrix

| Feature | Crate | Default | UEFI chain | Effect |
|---------|-------|---------|------------|--------|
| `vmx-launch` | `hv-hypervisor-boot` | off | off | VMCS fields + VMLAUNCH orchestration |
| `vmx-launch` | `hv-hypervisor-efi` | off | opt-in | VMX launch boot entry + markers |
| `vmx-launch` | `hv-hypervisor-efi-bin` | off | opt-in | Builds on `real-hw-execution` |
| `vmx-launch` | `hv-x86-cpu` | off | off | `execute_vmcs_field_programming`, `execute_vmlaunch` |

## Verification

- `cargo test --workspace` — pass
- `cargo test -p hv-hypervisor-boot --features vmx-launch` — pass
- `cargo test -p hv-hypervisor-efi --features vmx-launch` — pass
- `cargo test -p hv-x86-cpu --features execute-instructions,std,firmware-live-execution,vmx-launch` — pass
- `cargo clippy --all-targets --all-features -- -D warnings` — pass
- `cargo xtask coverage` — run in CI/local (multi-pass includes `vmx-launch`)
- `cargo xtask build-boot-chain-live` — builds with `real-hw-execution,vmx-launch`
- `cargo xtask live-qemu-smoke` — pass or skip (exit 0) when KVM/VMX unavailable

## Review status

Phase 16 closes the VMX launch gap from Phase 15 and establishes the smallest provable guest datapath (smoke guest image, boot info blob, VMLAUNCH seam). Full Gate D datapath (e1000, IPC forwarding, malicious tests, performance) remains deferred.
