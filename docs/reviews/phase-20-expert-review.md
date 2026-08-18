# Phase 20 expert review

Multi-domain review of Gate D datapath guests: ELF64 guest image parsing, reference partition ELFs, multi-partition VMLAUNCH orchestration, and `datapath-guests` feature chain (`cursor/phase-20-datapath-guests-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| ELF64 parsing | `parse_elf64`, `GuestElfImage`, build-time minimal ET_EXEC images |
| Reference guest images | `GUEST_{IN,MID,OUT}_ELF`, partition serial markers |
| Resident ELF install | `install_guest_elf` |
| Multi-partition launch planning | `plan_vmx_launch_all_partitions`, VM-id fallback for layout snapshots |
| Multi-partition CPU seams | `run_multi_vmx_launch_cpu_seam` |
| Gate D guests orchestration | `GateDDatapathGuestsResult`, `boot_*_gate_d_datapath_guests*()` |
| UEFI + xtask | `datapath-guests` feature chain, serial markers, coverage pass |

## Phase 19 deferrals closed

| Phase 19 item | Phase 20 disposition |
|---------------|---------------------|
| Real guest ELF images | **Partially closed** — minimal ELF64 ET_EXEC stubs generated at build time |
| Multi-partition VMLAUNCH | **Partially closed** — in/mid/out launch seams with validate-only default |
| 200 Mbit/s performance benchmark | **Deferred** (Phase 21+) |
| Live guest IPC runtime | **Unchanged** — host mock path only |
| DMAR MMIO | **Unchanged** |

## Feature matrix

| Feature | Crate | Default | UEFI chain | Effect |
|---------|-------|---------|------------|--------|
| `datapath-guests` | `hv-hypervisor-boot` | off | off | Gate D guests orchestration atop `datapath-malicious` |
| `datapath-guests` | `hv-x86-cpu` | off | off | `install_guest_elf`, `run_multi_vmx_launch_cpu_seam` |
| `datapath-guests` | `hv-hypervisor-efi` | off | opt-in | Guests boot entry + ELF/VMLAUNCH markers |
| `datapath-guests` | `hv-hypervisor-efi-bin` | off | opt-in | Builds on `datapath-malicious` |

## Verification

- `cargo test --workspace` — pass
- `cargo test -p hv-guest-boot` — pass (ELF parse + partition images)
- `cargo test -p hv-hypervisor-boot --features datapath-guests` — pass
- `cargo test -p hv-hypervisor-efi --features datapath-guests` — pass
- `cargo clippy --all-targets --all-features -- -D warnings` — pass
- `cargo xtask coverage` — multi-pass includes `datapath-guests`
- `cargo xtask build-boot-chain-live` — builds with full `datapath-*` chain including `datapath-guests`

## Review status

Phase 20 closes the guest ELF and multi-partition VMLAUNCH scaffolding gap from Phase 19: build-time reference ELF images, host-side ELF install, per-partition VMCS + launch seams, and Gate D guests orchestration with validate-only CPU seams. Performance benchmarking and live guest datapath runtime remain deferred to Phase 21+.
