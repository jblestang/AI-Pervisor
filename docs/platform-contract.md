# Platform contract

## Flow

```text
Desired configuration (YAML)
        |
        v
PlatformRequirements
        |
        | compare (future Phase 8–9)
        v
ObservedPlatform
        |
        v
ValidatedPlatform
        |
        v
StaticPlatformIR
```

Phases 0–3 implement `PlatformRequirements` and `StaticIntentIR`. `ObservedPlatform` and `ValidatedPlatform` are future runtime components built from CPUID, ACPI, and the UEFI memory map.

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
| Runtime CPUID/ACPI/firmware ingestion | Not started (Phase 5+) |
