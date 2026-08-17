# Architecture

## Scope (Phases 0–3)

This workspace establishes the static configuration pipeline and foundational types required before any VMX, VT-d hardware, or datapath work begins.

## Crates

| Crate | Role |
|-------|------|
| `hv-types` | Strong newtypes and overflow-safe arithmetic |
| `hv-config-model` | YAML model, validation, normalization, platform requirements, static intent IR |
| `hv-boot-abi` | Loader to hypervisor boot ABI skeleton |
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
  -> config.sha256 + review artifacts
```

The runtime must consume only compiled artifacts. Partition names such as `in`, `mid`, and `out` exist only in YAML; Rust code iterates generic `for partition in config.partitions`.

## Architectural gates

- **Gate A (before UEFI):** types, config model, IR, tests
- **Gate B (before VMX):** boot path, ACPI, observed platform validation, planners
- **Gate C (before e1000):** EPT/VT-d/IRQ isolation and lifecycle
- **Gate D (before optimization):** end-to-end datapath and malicious tests

Phases 0–3 complete Gate A.
