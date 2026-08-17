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
| ACPI observation contract | Interim flattened table bytes from loader (RSDP walk in Phase 6+) |
| UEFI loader binary (`.efi`) | Not started (Phase 6+) |
| VMX/EPT/VT-d enablement | Not started |
