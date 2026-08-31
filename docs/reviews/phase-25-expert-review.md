# Phase 25 expert review

Multi-domain review of Gate D datapath guest execution: live VMX VMLAUNCH for source-tree guests with boot-info/RDI handoff (`cursor/phase-25-guest-execution-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Guest execution seam | `run_datapath_guest_execution_cpu_seam`, `DatapathGuestExecutionCpuSeamOutcome` |
| Runtime disposition | `apply_runtime_disposition`, `DatapathRuntimeDisposition::Executed` |
| Gate D guest-execution | `GateDDatapathGuestExecutionResult`, boot entry + marker chain |
| UEFI + xtask | `datapath-guest-execution` feature chain, serial markers, coverage pass |

## Phase 24 deferrals closed

| Phase 24 item | Phase 25 disposition |
|---------------|---------------------|
| Live VMX execution of source-tree guests | **Closed (scaffolding)** — guest execution seam attempts VMLAUNCH with programmed VMCS when live env ready |
| In-VM 200 Mbit/s measurement | **Unchanged** — host wall-clock harness retained |

## Feature matrix

| Feature | Crate | Effect |
|---------|-------|--------|
| `datapath-guest-execution` | `hv-x86-cpu` | `run_datapath_guest_execution_cpu_seam` |
| `datapath-guest-execution` | `hv-hypervisor-boot` | Gate D guest-live + live execution orchestration |
| `datapath-guest-execution` | `hv-hypervisor-efi` | Guest-execution boot entry + execution marker |

## Serial markers

- `GATE_D_GUEST_EXECUTION_MARKER` — live VMX guest code executed for all partitions (ring-0 firmware only)
- Inherited markers from Phase 24 (boot-info, source ELF, runtime frame forward, etc.)

## Verification

- `cargo xtask build-guests`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-execution`
- `cargo test -p hv-hypervisor-efi --features datapath-guest-execution`
- `cargo clippy -p hv-hypervisor-boot -p hv-hypervisor-efi -p hv-x86-cpu -p hv-vmx --features datapath-guest-execution -- -D warnings`

## Review status

Phase 25 closes the live VMX guest execution scaffolding gap from Phase 24. Host/CI tests remain validate-only; ring-0 firmware with live execution enabled may reach `CpuInstructionDisposition::Executed`. In-VM throughput measurement remains deferred to Phase 26+.
