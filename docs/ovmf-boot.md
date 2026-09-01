# OVMF boot integration

Phase 11 extends the boot chain: the hypervisor embeds both a requirements snapshot and a compact layout snapshot, then runs full Gate C validation plus mock-backed VMX/EPT/VT-d init before returning `EFI_SUCCESS`.

## Build the boot chain

```bash
cargo xtask build-boot-chain
```

This:

1. Generates configuration artifacts under `build/` (including `config.sha256`)
2. Embeds the digest, requirements snapshot, and layout snapshot into the hypervisor image
3. Builds `build/boot-chain/hv-loader.efi` and `build/boot-chain/hv-hypervisor.efi` for `x86_64-unknown-uefi`

Individual images:

```bash
cargo xtask build-efi
cargo xtask build-hypervisor-efi
```

## Manual OVMF smoke boot (QEMU)

Install OVMF (example on Debian/Ubuntu):

```bash
sudo apt-get install ovmf
```

Create a temporary ESP and copy the boot chain:

```bash
mkdir -p /tmp/hv-esp/EFI/BOOT
cp build/boot-chain/hv-loader.efi /tmp/hv-esp/EFI/BOOT/BOOTX64.EFI
cp build/boot-chain/hv-hypervisor.efi /tmp/hv-esp/hv-hypervisor.efi
```

Run QEMU with OVMF and the ESP attached:

```bash
qemu-system-x86_64 \
  -machine q35,accel=kvm:tcg \
  -cpu max \
  -m 8192 \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE.fd \
  -drive if=pflash,format=raw,file=/tmp/OVMF_VARS.fd \
  -drive format=raw,file=fat:rw:/tmp/hv-esp \
  -serial stdio
```

On success the loader chain-loads the hypervisor, which validates the published transfer blob, runs platform observation and fail-closed requirements compare, reconstructs static layout from the embedded layout snapshot, and invokes mock VMX/EPT/VT-d backends (Phase 11 Gate C). The hypervisor then returns `EFI_SUCCESS`. OVMF then returns to the Boot Manager menu (expected for UEFI apps that exit successfully).

Automated verification (uses `configs/ovmf-smoke.yaml`, which relaxes VMX/EPT/VT-d requirements for TCG-backed QEMU; host tests cover Gate C with production `configs/qemu.yaml`):

```bash
cargo xtask ovmf-smoke-boot
```

This builds the boot chain (unless `--no-build` is passed), launches OVMF/QEMU with a temporary ESP, and checks the firmware serial log for a successful boot attempt without an `Aborted` status. CI runs `cargo xtask ovmf-smoke-boot --no-build` after `build-boot-chain`.

## REAL_HW live smoke boot (KVM)

Build the REAL_HW boot chain (hypervisor with `real-hw-execution` through `datapath-guest-relay-live`; runs `cargo xtask build-guests` first to embed source-tree ELFs):

```bash
cargo xtask build-boot-chain-live
```

Run a KVM-backed smoke boot (requires `/dev/kvm`, host VMX, OVMF, and QEMU). Exits 0 with a skip message when nested KVM or VMX is unavailable:

```bash
cargo xtask live-qemu-smoke
```

For a real experiment without validate-only mock proof, require in-VM executed markers and fail when the host cannot run nested VMX/OVMF:

```bash
cargo xtask live-qemu-smoke --require-executed --no-skip --build
```

When `configs/qemu.yaml` declares `qemu.network.enabled: true` (the default production config), live smoke attaches outer-QEMU e1000 devices at the contract BDFs with user-mode netdev backends (`net_in`, `net_out`). Pass `--no-host-net` to force the legacy `-net none` launch.

```bash
# Outer QEMU e1000 at 0000:00:03.0 / 0000:00:04.0 with user netdev (from config)
cargo xtask live-qemu-smoke --build

# Legacy launch without host-visible NICs
cargo xtask live-qemu-smoke --no-host-net --build
```

The strict path runs an OVMF/KVM serial probe first; hosts where OVMF produces no serial output under KVM (common on broken nested-virt cloud VMs) fail fast instead of timing out with an empty log.

On success the serial log includes Gate D guest relay live markers (`Gate D: guest source ELF installed…`, `Gate D: guest boot info installed…`, `Gate D: guest throughput target 200 Mbit/s met`, `hypervisor Gate D datapath guest throughput succeeded`) and at least one REAL_HW VMX marker (`REAL_HW: VMXON Executed`, `REAL_HW: EPT pointer Executed`, or `REAL_HW: VMLAUNCH Executed`). When live in-VM measurement completes (Phase 29+), the log also includes `Gate D: guest throughput measured under live VMX` and guest `GUEST: datapath relay benchmark complete`. With `--require-executed`, all three REAL_HW VMX markers plus `Gate D: guest source-tree code executed under VMX for all partitions` are required. Legacy `vmx-launch`-only firmware may still emit `hypervisor Gate C REAL_HW boot succeeded`.

## PCI enumeration limits (Phase 8)

Firmware PCI discovery uses legacy CF8/CFC config ports on segment 0. It walks bus numbers starting at 0 and stops at the first bus with no responding devices. It does not recurse PCI-to-PCI bridges or enumerate ECAM/MMCONFIG. This is sufficient for the reference QEMU q35 topology (`0000:00:03.0`, `0000:00:04.0`) but is not production-complete firmware discovery.

## Runtime inputs collected by the loader

| Input | Source |
|-------|--------|
| UEFI memory map | `GetMemoryMap` via `uefi::boot::memory_map` |
| ACPI RSDP | UEFI configuration table (`ACPI2_GUID` / `ACPI_GUID`) |
| CPUID snapshot | Raw `cpuid` instructions |
| PCI BDF list | PCI config-space scan via legacy ports (segment 0, bus 0 walk; see limits below) |
| ACPI tables | RSDP-directed walk via identity-mapped physical memory |

## Transfer handoff

| Artifact | Role |
|----------|------|
| `HypervisorTransferHeader` | Magic/version header for transfer blob (ABI v2 includes `published_alloc_size`) |
| Boot info blob | Versioned loader handoff (`HVBOOT`) |
| Observation payload | CPUID, PCI list, memory map, ACPI tables |
| `HV_TRANSFER_TABLE_GUID` | UEFI configuration-table entry pointing at the transfer header |

## Crates

| Crate | Role |
|-------|------|
| `hv-observation-types` | `no_std` observation input types shared by loader and platform model |
| `hv-loader-efi-bin` | UEFI loader application (`hv-loader.efi`) |
| `hv-loader-efi` | Portable handoff + transfer helpers used by host tests and firmware |
| `hv-hypervisor-efi-bin` | UEFI hypervisor application (`hv-hypervisor.efi`) |
| `hv-hypervisor-efi` | Portable Gate C boot + mock or REAL_HW init entry (`real-hw-execution` feature) |
| `hv-hypervisor-boot` | Portable observe/validate/Gate C orchestration (`no_std` + `alloc`) |
| `hv-vmx` | VMX init plan and backend abstraction (mock backend) |
| `hv-ept` | EPT init plan and backend abstraction (mock backend) |
| `hv-vtd` | VT-d init plan and backend abstraction (mock backend) |
| `hv-x86-cpu` | Host-only CPUID probes, CPU instruction seams, resident install, live asm (not linked into default UEFI images) |
