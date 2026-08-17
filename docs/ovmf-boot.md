# OVMF boot integration

Phase 7 delivers a buildable UEFI loader application (`hv-loader.efi`) that collects firmware inputs at runtime and runs the portable loader handoff path.

## Build the loader image

```bash
cargo xtask build-efi
```

This:

1. Generates configuration artifacts under `build/` (including `config.sha256`)
2. Embeds the digest into the loader image at compile time
3. Builds `build/hv-loader.efi` for `x86_64-unknown-uefi`

## Manual OVMF smoke boot (QEMU)

Install OVMF (example on Debian/Ubuntu):

```bash
sudo apt-get install ovmf
```

Create a temporary ESP and copy the loader:

```bash
mkdir -p /tmp/hv-esp/EFI/BOOT
cp build/hv-loader.efi /tmp/hv-esp/EFI/BOOT/BOOTX64.EFI
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

On success the loader returns to firmware with `EFI_SUCCESS` after building the boot-info handoff in memory. Hypervisor transfer and PCI enumeration remain follow-on work.

## Runtime inputs collected by the loader

| Input | Source |
|-------|--------|
| UEFI memory map | `GetMemoryMap` via `uefi::boot::memory_map` |
| ACPI RSDP | UEFI configuration table (`ACPI2_GUID` / `ACPI_GUID`) |
| CPUID snapshot | Raw `cpuid` instructions |
| PCI BDF list | Not yet enumerated (empty in Phase 7) |
| ACPI tables | RSDP-directed walk via identity-mapped physical memory |

## Crates

| Crate | Role |
|-------|------|
| `hv-observation-types` | `no_std` observation input types shared by loader and platform model |
| `hv-loader-efi-bin` | UEFI application binary (`hv-loader.efi`) |
| `hv-loader-efi` | Portable handoff entry used by host tests and firmware |
