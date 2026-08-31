# Phase 25 expert review

Multi-domain review of Gate D datapath guest execution: live VMX VMLAUNCH for source-tree guests with boot-info/RDI handoff (`cursor/phase-25-guest-execution-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Guest execution seam | `run_datapath_guest_execution_cpu_seam`, `DatapathGuestExecutionCpuSeamOutcome` |
| Runtime disposition | `apply_runtime_disposition`, `runtime_disposition_for_guest_execution_seam` |
| Gate D guest-execution | `GateDDatapathGuestExecutionResult`, `build_guest_live_vmcs_fields`, boot entry + marker chain |
| UEFI + xtask | `datapath-guest-execution` feature chain, serial markers, coverage pass |

## Phase 24 deferrals closed

| Phase 24 item | Phase 25 disposition |
|---------------|---------------------|
| Live VMX execution of source-tree guests | **Closed (scaffolding)** — guest execution seam attempts VMLAUNCH with programmed VMCS when live env ready |
| In-VM 200 Mbit/s measurement | **Unchanged** — host wall-clock harness retained |

## Issues found and fixed

| Issue | Fix |
|-------|-----|
| Duplicate VMCS field programming in guest init vs execution | Added shared `build_guest_live_vmcs_fields` helper used by both paths |
| Ad-hoc CPU disposition → runtime disposition mapping | Added `runtime_disposition_for_guest_execution_seam` in `hv-datapath` |
| No post-seam consistency checks | Gate D validates Executed implies full VMLAUNCH count and runtime disposition alignment |
| Partition launch records stale after execution | Update per-partition `launch_seam` to `Executed` when guest execution seam succeeds |
| Misleading `partitions_launched` seam field name | Renamed to `partitions_validated` to match runtime seam terminology |
| Weak unit/integration tests | Added hv-datapath disposition tests, hv-x86-cpu guest execution seam tests, marker consistency assertion |

## Feature matrix

| Feature | Crate | Default | UEFI chain | Effect |
|---------|-------|---------|------------|--------|
| `datapath-guest-execution` | `hv-x86-cpu` | off | off | `run_datapath_guest_execution_cpu_seam` |
| `datapath-guest-execution` | `hv-hypervisor-boot` | off | off | Gate D guest-live + live execution orchestration |
| `datapath-guest-execution` | `hv-hypervisor-efi` | off | opt-in | Guest-execution boot entry + execution marker |

## Serial markers

- `GATE_D_GUEST_EXECUTION_MARKER` — live VMX guest code executed for all partitions (ring-0 firmware only)
- Inherited markers from Phase 24 (boot-info, source ELF, runtime frame forward, etc.)

## Verification

- `cargo xtask build-guests`
- `cargo test -p hv-x86-cpu --features datapath-guest-execution`
- `cargo test -p hv-datapath runtime_disposition_for_guest_execution_seam`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-execution`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-live`
- `cargo test -p hv-hypervisor-efi --features datapath-guest-execution`
- `cargo clippy -p hv-hypervisor-boot -p hv-hypervisor-efi -p hv-x86-cpu -p hv-vmx --features datapath-guest-execution -- -D warnings`

## Review status

Phase 25 closes the live VMX guest execution scaffolding gap from Phase 24 with shared VMCS programming, consistent disposition mapping, and stronger validation/tests. Host/CI tests remain validate-only; ring-0 firmware with live execution enabled may reach `CpuInstructionDisposition::Executed`. In-VM throughput measurement remains deferred to Phase 26+.
