# Architecture

## Scope

| Phase | Gate | Deliverables |
|-------|------|--------------|
| 0–3 | A (before UEFI) | Types, config model, static intent IR, ABI skeletons, tests |
| 4 | B (start) | Observed platform model, fail-closed validation, static platform IR planner |
| 5 | B | Boot info ABI parsing, runtime CPUID/ACPI/UEFI observation, loader handoff, hypervisor boot check |
| 6 | B | ACPI RSDP walk, firmware memory model, portable UEFI loader entry (`hv-loader-efi`) |
| 7 | B | UEFI `.efi` binary build, runtime firmware input collection, OVMF docs |
| 8 | B | Hypervisor transfer ABI, UEFI chain-load, PCI enumeration, requirements snapshot embedding |
| 9 | B | Transfer ABI v2 hardening, VMX init foundation (mock backend), Gate B closure on UEFI hypervisor path |
| 10 | C (start) | EPT/VT-d init planning, mock backends, host-tested Gate C orchestration |
| 11 | C | Embedded layout snapshot, UEFI Gate C closure (mock VMX/EPT/VT-d) |
| 12 | C | Hardware programming backends (VMXON/EPT/VT-d structure encoding, host-tested) |
| 13 | C | CPU instruction seams (CPUID + validate-only disposition, host-tested) |
| 14 | C | Live VMX/EPT/VT-d instruction modules with runtime + ring-0 gates (host-tested) |
| 15 | C | REAL_HW resident install, VMCS prepare, firmware REAL_HW path, KVM live smoke harness |
| 16+ | C–D | VMX launch, guest datapath |

Phases 0–3 complete Gate A. Phase 4 begins Gate B with host-side platform validation and deterministic layout planning.

## Crates

| Crate | Role |
|-------|------|
| `hv-types` | Strong newtypes and overflow-safe arithmetic |
| `hv-config-model` | YAML model, validation, normalization, platform requirements, static intent IR |
| `hv-platform-model` | Observed platform validation, runtime observation, static platform IR planning |
| `hv-acpi-walk` | RSDP → XSDT/RSDT ACPI table discovery in firmware memory |
| `hv-boot-abi` | Loader to hypervisor boot ABI (header, descriptors, requirements/layout snapshots, parse-only views) |
| `hv-loader` | Boot info blob construction, firmware memory fixtures, handoff bundle |
| `hv-loader-efi` | Portable UEFI loader entry (`uefi_loader_entry`) |
| `hv-loader-efi-bin` | UEFI application binary (`hv-loader.efi`) |
| `hv-hypervisor-efi` | Portable hypervisor transfer verification entry |
| `hv-hypervisor-efi-bin` | UEFI hypervisor application binary (`hv-hypervisor.efi`) |
| `hv-observation-types` | Boot-time observation input types (`no_std`) |
| `hv-hypervisor-boot` | Portable Gate B boot validation and VMX/EPT/VT-d init orchestration (`no_std` + `alloc`) |
| `hv-vmx` | VMX init plan and backend abstraction (mock backend) |
| `hv-ept` | EPT init plan and backend abstraction (mock backend; Phase 10) |
| `hv-vtd` | VT-d init plan and backend abstraction (mock backend; Phase 10) |
| `hv-x86-cpu` | x86 CPUID probes and CPU instruction seams for Gate C (host-only; Phase 13) |
| `hv-hypervisor` | Host re-exports over `hv-hypervisor-boot` |
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

Phases 0–3 complete Gate A. Phase 4 adds observed-platform validation and static layout planning (Gate B foundation). Phase 5 wires the boot path: the loader builds a versioned boot info blob, the hypervisor parses it, observes firmware inputs, and runs fail-closed platform validation before VMX setup. Phase 6 replaces the interim flattened ACPI contract with RSDP-directed table discovery and introduces the portable UEFI loader entry crate. Phase 7 builds the UEFI application (`hv-loader.efi`) that collects runtime firmware inputs and runs the handoff under OVMF. Phase 8 publishes the hypervisor transfer blob, chain-loads `hv-hypervisor.efi`, and enumerates PCI devices at firmware boot. Phase 9 closes Gate B on the UEFI hypervisor path: transfer ABI v2 binds loader allocation size, the hypervisor runs full observe/validate plus mock-backed VMX init, and `hv-vmx`/`hv-hypervisor-boot` split portable orchestration from host tests. Phase 10 begins Gate C foundation: `hv-ept` and `hv-vtd` mirror the VMX planning seam with mock backends, and `boot_from_transfer_and_init_gate_c()` chains VMX + EPT + VT-d init on the host path. Phase 11 closes Gate C on the UEFI hypervisor path: `LayoutSnapshot` is embedded alongside the requirements snapshot, and firmware runs full Gate C mock init via `boot_from_transfer_and_init_gate_c_from_snapshots()`. Phase 12 adds hardware programming backends that encode VMXON, EPT, and VT-d structures on the host path (`Programming*Backend`, `boot_*_gate_c_programming*()`); UEFI remains mock-backed until firmware-safe programming buffers land. Phase 13 adds host-only CPU instruction seams in `hv-x86-cpu`: CPUID capability probes, validate-only instruction disposition, and `CpuSeam*Backend` orchestration via `boot_*_gate_c_cpu_seam*()` (`cpu-seams` feature); UEFI remains mock-backed. Phase 14 adds live privileged instruction modules (`execute-instructions`): VMXON, EPT pointer VMWRITE, and VT-d enable intent behind `HV_X86_LIVE_INSTRUCTIONS=1` and ring-0 gates, with `boot_*_gate_c_live_execution*()` host orchestration (`live-execution` feature). Phase 15 adds REAL_HW resident page installation (`PageAllocator`, `install_*` helpers), VMCS prepare (VMCLEAR/VMPTRLD), firmware-safe live opt-in (`firmware-live-execution`), Gate C REAL_HW orchestration (`real-hw-execution`), UEFI REAL_HW boot entry with serial markers, and a KVM/QEMU live smoke harness (`cargo xtask live-qemu-smoke`). All parsing surfaces are fuzzed via libFuzzer (`fuzz/`, `cargo xtask fuzz`); see [fuzzing.md](fuzzing.md). OVMF boot: [ovmf-boot.md](ovmf-boot.md).

## No-panic policy

Production code must not panic. See [no-panic.md](no-panic.md). Enforced by workspace and crate-level Clippy denies plus explicit error propagation in CLIs.
