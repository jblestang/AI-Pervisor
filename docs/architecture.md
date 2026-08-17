# Architecture

## Scope

| Phase | Gate | Deliverables |
|-------|------|--------------|
| 0–3 | A (before UEFI) | Types, config model, static intent IR, ABI skeletons, tests |
| 4 | B (start) | Observed platform model, fail-closed validation, static platform IR planner |
| 5 | B | Boot info ABI parsing, runtime CPUID/ACPI/UEFI observation, loader handoff, hypervisor boot check |
| 6 | B | ACPI RSDP walk, firmware memory model, portable UEFI loader entry (`hv-loader-efi`) |
| 7+ | B–D | UEFI `.efi` binary build, VMX/EPT/VT-d, datapath |

Phases 0–3 complete Gate A. Phase 4 begins Gate B with host-side platform validation and deterministic layout planning.

## Crates

| Crate | Role |
|-------|------|
| `hv-types` | Strong newtypes and overflow-safe arithmetic |
| `hv-config-model` | YAML model, validation, normalization, platform requirements, static intent IR |
| `hv-platform-model` | Observed platform validation, runtime observation, static platform IR planning |
| `hv-acpi-walk` | RSDP → XSDT/RSDT ACPI table discovery in firmware memory |
| `hv-boot-abi` | Loader to hypervisor boot ABI (header, descriptors, parse-only views) |
| `hv-loader` | Boot info blob construction, firmware memory fixtures, handoff bundle |
| `hv-loader-efi` | Portable UEFI loader entry (`uefi_loader_entry`) |
| `hv-hypervisor` | Boot-path orchestration (digest verify, observe, validate) |
| `hv-guest-abi` | Hypervisor to guest boot ABI skeleton |
| `hv-config` | Host-side configuration compiler CLI |
| `xtask` | Developer command wrappers |

## Configuration pipeline

```text
configs/qemu.yaml
  -> RawConfig
  -> syntax validation
  -> semantic validation
  -> NormalizedConfig
  -> PlatformRequirements
  -> StaticIntentIR
  -> StaticPlatformIR
  -> config.sha256 + review artifacts
```

The runtime must consume only compiled artifacts. Partition names such as `in`, `mid`, and `out` exist only in YAML; Rust code iterates generic `for partition in config.partitions`.

## Architectural gates

- **Gate A (before UEFI):** types, config model, IR, tests
- **Gate B (before VMX):** boot path, ACPI, observed platform validation, planners
- **Gate C (before e1000):** EPT/VT-d/IRQ isolation and lifecycle
- **Gate D (before optimization):** end-to-end datapath and malicious tests

Phases 0–3 complete Gate A. Phase 4 adds observed-platform validation and static layout planning (Gate B foundation). Phase 5 wires the boot path: the loader builds a versioned boot info blob, the hypervisor parses it, observes firmware inputs, and runs fail-closed platform validation before VMX setup. Phase 6 replaces the interim flattened ACPI contract with RSDP-directed table discovery and introduces the portable UEFI loader entry crate.

## No-panic policy

Production code must not panic. See [no-panic.md](no-panic.md). Enforced by workspace and crate-level Clippy denies plus explicit error propagation in CLIs.
