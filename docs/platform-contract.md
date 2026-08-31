# Platform contract

## Flow

```text
Desired configuration (YAML)
        |
        v
PlatformRequirements
        |
        | compare (Phase 5 runtime observation)
        v
ObservedPlatform
        |
        v
ValidatedPlatform
        |
        v
StaticPlatformIR
```

Phases 0–3 implement `PlatformRequirements` and `StaticIntentIR`. Phase 4 adds JSON fixture-based `ObservedPlatform` validation. Phase 5 adds runtime observation from CPUID, ACPI table bytes, the UEFI memory map, and PCI BDFs discovered at boot.

## PlatformRequirements

Derived deterministically from normalized configuration. Expresses at minimum:

- architecture (`x86_64`)
- VMX, EPT, VT-d, NX requirements
- minimum physical cores and RAM
- SMT policy
- interrupt remapping requirement
- optional/preferred features (x2APIC, invariant TSC, VPID, VMX preemption timer)
- required page sizes
- expected PCI devices and owning partitions

## Fail-closed rule

If observed hardware does not satisfy `PlatformRequirements`, boot must be refused. The hardware must never silently downgrade a required property.

## Current status

| Component | Status |
|-----------|--------|
| Desired configuration | Implemented (`configs/qemu.yaml`) |
| PlatformRequirements | Implemented |
| StaticIntentIR | Implemented |
| ObservedPlatform | Implemented (`hv-platform-model`, JSON fixtures for tests) |
| ValidatedPlatform | Implemented (fail-closed compare against requirements) |
| StaticPlatformIR with resolved addresses | Implemented (deterministic planner) |
| Runtime CPUID/ACPI/firmware ingestion | Implemented (`observe_platform`, loader handoff) |
| ACPI table discovery | Implemented (`hv-acpi-walk`: RSDP → XSDT/RSDT walk in firmware memory) |
| Portable UEFI loader entry | Implemented (`hv-loader-efi::uefi_loader_entry`, host-tested) |
| UEFI `.efi` binary build | Implemented (`cargo xtask build-efi` → `build/hv-loader.efi`) |
| OVMF runtime boot | Documented (`docs/ovmf-boot.md`; CI smoke via `cargo xtask ovmf-smoke-boot`) |
| UEFI hypervisor Gate B boot | Implemented (Phase 9: observe, validate, mock VMX init via `hv-hypervisor-efi`) |
| Transfer allocation binding | Implemented (Phase 9: `published_alloc_size` in transfer ABI v2) |
| VMX init foundation | Implemented (Phase 9: `hv-vmx` plan + mock backend; no hardware VMXON) |
| EPT init foundation | Implemented (Phase 10: `hv-ept` plan + mock backend; no EPT paging) |
| VT-d init foundation | Implemented (Phase 10: `hv-vtd` plan + mock backend; no IOMMU programming) |
| Gate C orchestration (host) | Implemented (Phase 10: `boot_from_transfer_and_init_gate_c()` with `StaticPlatformIR`) |
| Layout snapshot ABI | Implemented (Phase 11: `LayoutSnapshot` in `hv-boot-abi`) |
| UEFI Gate C boot | Implemented (Phase 11: embedded layout snapshot + mock VMX/EPT/VT-d init via `hv-hypervisor-efi`) |
| Gate C hardware programming (host) | Implemented (Phase 12: `Programming*Backend`, structure encoding without CPU instructions) |
| Gate C CPU instruction seams (host) | Implemented (Phase 13: `hv-x86-cpu`, `CpuSeam*Backend`, validate-only disposition via `cpu-seams` feature) |
| Gate C live instruction execution (host) | Implemented (Phase 14: `execute-instructions`, runtime + ring-0 gates, `boot_*_gate_c_live_execution*`) |
| REAL_HW resident install + VMCS prepare | Implemented (Phase 15: `PageAllocator`, `ResidentCpuSeam*Backend`, `execute_vmcs_prepare`, `real-hw-execution`) |
| REAL_HW VMX launch under KVM/OVMF | Implemented (Phase 16: `vmx-launch`, `build-boot-chain-live`, `live-qemu-smoke`; skips without nested KVM) |
| Gate D guest relay live under KVM/OVMF | Implemented (Phase 28: validate-only smoke; Phase 29: measurement tier with boot-info counters + executed markers) |
| DMAR MMIO / guest datapath | Partial (Phases 18–29: synthetic IPC forward + e1000 MMIO smoke, mock/wall-clock benchmark, guest-runtime relay, freestanding source-tree guests with sustained relay loops + counter tails, boot-info/RDI handoff, live VMX guest execution scaffolding, in-VM throughput measurement via boot-info counters; full VM-exit/resume relay loop deferred) |
