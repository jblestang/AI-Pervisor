# Phase 24 expert review

Multi-domain review of Gate D datapath guest live: boot-info blob install, VMCS `GUEST_RDI` handoff, and `datapath-guest-live` feature chain (`cursor/phase-24-guest-live-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| VMCS guest entry | `VMCS_GUEST_RDI`, `patch_guest_boot_info_rdi`, `guest_boot_info_rdi_programmed` |
| Boot-info install | `install_guest_elf_with_boot_info`, Gate D guest launch loop |
| Gate D guest-live | `GateDDatapathGuestLiveResult`, `boot_*_gate_d_datapath_guest_live*()` |
| UEFI + xtask | `datapath-guest-live` feature chain, serial markers, coverage pass |

## Phase 23 deferrals closed

| Phase 23 item | Phase 24 disposition |
|---------------|---------------------|
| Boot-info blob install + `GUEST_RDI` | **Closed** — single allocation for ELF + boot-info, VMCS RDI patch validated |
| Live VMX execution of source-tree guests | **Unchanged** — CPU seams remain validate-only |
| In-VM 200 Mbit/s measurement | **Unchanged** — host wall-clock harness retained |

## Issues found and fixed

| Issue | Fix |
|-------|-----|
| Boot-info colocated install could land in VMCS allocation | Replaced split install with `install_guest_elf_with_boot_info` sizing one allocation for ELF tail + boot-info |
| Duplicate boot-info rebuild in guest launch loop | Reuse `malicious.live.foundation.partition_boot_infos` from datapath foundation |
| No VMCS RDI programming check | Added `guest_boot_info_rdi_programmed` validation before multi-partition launch |
| Weak integration tests | Added resident + VMCS unit tests; guest-live test asserts boot-info phys follows entry |

## Feature matrix

| Feature | Crate | Default | UEFI chain | Effect |
|---------|-------|---------|------------|--------|
| `datapath-guest-live` | `hv-hypervisor-boot` | off | off | Gate D guest-sources + boot-info install/RDI |
| `datapath-guest-live` | `hv-x86-cpu` | off | off | `install_guest_elf_with_boot_info` |
| `datapath-guest-live` | `hv-hypervisor-efi` | off | opt-in | Guest-live boot entry + boot-info marker |

## Serial markers

- `GATE_D_GUEST_BOOT_INFO_INSTALLED_MARKER` — boot-info installed and RDI patched for all partitions
- Inherited markers from Phase 23 (`GATE_D_GUEST_SOURCE_ELF_MARKER`, runtime markers, etc.)

## Verification

- `cargo xtask build-guests`
- `cargo test -p hv-x86-cpu --features datapath-guest-live`
- `cargo test -p hv-vmx patch_guest_boot_info_rdi_programs_guest_rdi_field`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-live`
- `cargo test -p hv-hypervisor-efi --features datapath-guest-live`
- `cargo clippy -p hv-hypervisor-boot -p hv-hypervisor-efi -p hv-x86-cpu -p hv-vmx --features datapath-guest-live -- -D warnings`

## Review status

Phase 24 closes the boot-info handoff gap from Phase 23: per-partition blobs are installed in the same resident allocation as source ELFs, VMCS guest RDI is patched and validated, and Gate D/UEFI orchestration matches prior datapath phases. Live VMX guest code execution and in-VM throughput measurement remain deferred to Phase 25+.
