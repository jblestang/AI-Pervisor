# Phase 24 expert review

Multi-domain review of Gate D datapath guest live: boot-info blob install, VMCS `GUEST_RDI` handoff, and `datapath-guest-live` feature chain (`cursor/phase-24-guest-live-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| VMCS guest entry | `VMCS_GUEST_RDI`, `patch_guest_boot_info_rdi` |
| Boot-info install | `install_guest_boot_info_colocated`, Gate D guest launch loop |
| Gate D guest-live | `GateDDatapathGuestLiveResult`, `boot_*_gate_d_datapath_guest_live*()` |
| UEFI + xtask | `datapath-guest-live` feature chain, serial markers, coverage pass |

## Phase 23 deferrals closed

| Phase 23 item | Phase 24 disposition |
|---------------|---------------------|
| Boot-info blob install + `GUEST_RDI` | **Closed** — colocated install after ELF + VMCS RDI patch for all partitions |
| Live VMX execution of source-tree guests | **Unchanged** — CPU seams remain validate-only |
| In-VM 200 Mbit/s measurement | **Unchanged** — host wall-clock harness retained |

## Feature matrix

| Feature | Crate | Default | UEFI chain | Effect |
|---------|-------|---------|------------|--------|
| `datapath-guest-live` | `hv-hypervisor-boot` | off | off | Gate D guest-sources + boot-info install/RDI |
| `datapath-guest-live` | `hv-x86-cpu` | off | off | `install_guest_boot_info_colocated` |
| `datapath-guest-live` | `hv-hypervisor-efi` | off | opt-in | Guest-live boot entry + boot-info marker |

## Serial markers

- `GATE_D_GUEST_BOOT_INFO_INSTALLED_MARKER` — boot-info installed and RDI patched for all partitions
- Inherited markers from Phase 23 (`GATE_D_GUEST_SOURCE_ELF_MARKER`, runtime markers, etc.)

## Verification

- `cargo xtask build-guests`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-live`
- `cargo test -p hv-hypervisor-efi --features datapath-guest-live`
- `cargo clippy -p hv-hypervisor-boot -p hv-hypervisor-efi -p hv-x86-cpu --features datapath-guest-live -- -D warnings`

## Review status

Phase 24 closes the boot-info handoff gap from Phase 23: per-partition blobs are copied colocated with installed source ELFs and wired into VMCS guest RDI before launch. Live VMX guest code execution and in-VM throughput measurement remain deferred to Phase 25+.
