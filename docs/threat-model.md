# Threat model (stub)

Phases 0–3 establish documentation scaffolding only. This stub defines trust boundaries to be expanded in later phases.

## Trusted components (initial)

- Operator-supplied validated YAML consumed through `hv-config`
- Host-side configuration compiler and test tooling
- Compiled configuration digest verified at boot (future)

## Potentially hostile inputs

- Malformed or hostile YAML
- Corrupted guest images (future)
- Compromised guest partitions (Phase 19: host-simulated IPC integrity tests)
- Malicious PCI DMA (future)
- Malformed network data (future)
- Malformed ACPI/firmware tables (future)

## Explicitly out of scope for MVP

- Physical attacker
- Supply-chain compromise of build hosts (tracked separately)
- SMM/firmware behavior beyond documentation and measurement
- Absolute temporal isolation across shared caches/DRAM

## Assets to protect (tracked)

- Hypervisor integrity and confidentiality
- Private partition RAM
- IPC integrity
- PCI device ownership
- EPT and VT-d tables
- VMCS state
- Boot manifest and configuration digest

See also future docs: `safety-model.md`, `fault-model.md`, `timing-model.md`.
