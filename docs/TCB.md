# Trusted Computing Base (TCB)

Issue: 0.1 — Task 6 (`hv` VT‑x / EPT bring-up)  
Scope: intentional `unsafe` and residual hardware trust for the edge-gateway spike.

Normative design: system SSDD §8; Verus covers safe `isolation` / `ring` models only.

## Policy

| Rule | Detail |
|------|--------|
| List every intentional `unsafe` | This document + source `SAFETY:` comments |
| No silent TCB growth | New `unsafe` in datapath requires an entry here |
| Coverage exclusions | Only files listed below may be omit-listed in `xtask/coverage-allowlist.toml` (Task 8 — each exclusion needs a one-line justification; see that file) |

## Inventory

| ID | Location | Hazard | Residual trust |
|----|----------|--------|----------------|
| TCB-VMX-001 | `hv/src/vmx/vmxon.rs` (`rdmsr` / `wrmsr` / `cpuid` / CR0·CR4 / `vmxon`) | Wrong MSR/CR sequencing | CPU implements SDM VMX |
| TCB-VMX-002 | `hv/src/vmx/vmcs.rs` `hw::{vmclear,vmptrld,vmread,vmwrite,vmlaunch,vmresume}` | Corrupted guest/host state | VMCS encodings match CPU |
| TCB-VMX-003 | `hv/src/main.rs` Multiboot/`FlatHvConfig` bring-up + identity GPA `slice_at` | Invalid pointer / wrong GPA | Task 7 loader ABI + trampoline 1 GiB map |
| TCB-VMX-004 | `hv/src/vmx/enter.rs` freestanding `enter_guest` / host TSS·GDT / launch trampoline / GPR capture | Wrong host RIP/TR or guest state | Caps-adjusted controls; nested VT‑x lab |
| TCB-UART-001 | `hv/src/main.rs` `platform_uart_write` (`out dx, al`) | Wrong I/O port | Port from `FlatHvConfig.uart_io_port` |
| TCB-EPT-001 | Freestanding EPT page install / EPTP `vmwrite` | Stale HPA / wrong leaf bits | Intent model in `hv::ept` is safe + tested; **walk** is hardware |
| TCB-GUEST-001 | `guest-wan` / `guest-lan` `_start` deref of `*const BootInfo` + GPA slices | Invalid boot blob / unmapped GPA | `hv` filled blob + guest PT + EPT |
| TCB-GUEST-002 | `guest-common` bump allocator (`GlobalAlloc`) | Heap overrun | Fixed bootstrap size; not lab GPA |
| TCB-NIC-001 | `pci-vf` Sriov MMIO (`from_mmio_tips` / `mmio_read_u32` when window present) | Device poke | IOMMU + VFIO + config `nic_mmio_*` (`HARNESS.md`) |

## Explicitly *not* TCB (assurance candidates)

- `isolation::MemoryMap` / `Region` / `Perms` (Verus-oriented)
- `ring::SpscRing` push/pop
- `hv::guest_mem::build_layout`, `hv::ept::build_ept_intent`, `classify_gpa_fault`
- `hv::boot_blob::fill_boot_info` (incl. config-driven `nic_mmio_*`)
- `hv::vmx::exit::handle_exit` policy (safe Rust; host-tested)
- `hv::vmx::controls` / `guest_pt` / `enter::prepare_launch_image` (host-tested; no VT-x)
- `hv::l2` prepare helpers (host-tested)
- `guest-common` forward datapath + Inject queues

## Unsafe count discipline

When editing TCB files, keep `SAFETY:` comments adjacent to each `unsafe` block describing the invariant the caller upholds.
