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
| Real VMXON/EPT/VT-d enablement | Not started (Gate C) |
