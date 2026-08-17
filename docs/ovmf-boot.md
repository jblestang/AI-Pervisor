# OVMF boot integration

Phase 8 delivers a buildable UEFI boot chain: the loader (`hv-loader.efi`) collects firmware inputs, builds the hypervisor transfer blob, publishes it through the UEFI configuration table, and chain-loads the hypervisor image (`hv-hypervisor.efi`).

## Build the boot chain

```bash
cargo xtask build-boot-chain
```

This:

1. Generates configuration artifacts under `build/` (including `config.sha256`)
2. Embeds the digest into the loader image and digest + requirements snapshot into the hypervisor image
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
  -m 4096 \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE.fd \
  -drive if=pflash,format=raw,file=/tmp/OVMF_VARS.fd \
  -drive format=raw,file=fat:rw:/tmp/hv-esp \
  -serial stdio
```

On success the loader chain-loads the hypervisor, which verifies the published transfer blob and returns `EFI_SUCCESS`.

## Runtime inputs collected by the loader

| Input | Source |
|-------|--------|
| UEFI memory map | `GetMemoryMap` via `uefi::boot::memory_map` |
| ACPI RSDP | UEFI configuration table (`ACPI2_GUID` / `ACPI_GUID`) |
| CPUID snapshot | Raw `cpuid` instructions |
| PCI BDF list | PCI config-space scan (segment 0) |
| ACPI tables | RSDP-directed walk via identity-mapped physical memory |

## Transfer handoff

| Artifact | Role |
|----------|------|
| `HypervisorTransferHeader` | Magic/version header for the transfer blob |
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
| `hv-hypervisor-efi` | Portable transfer verification entry used by host tests and firmware |
