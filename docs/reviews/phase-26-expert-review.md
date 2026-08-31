# Phase 26 expert review

Multi-domain review of Gate D in-VM guest throughput benchmark: mock guest-runtime relay measurement with live VMX seam opt-in (`cursor/phase-26-guest-throughput-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Guest throughput benchmark | `run_mock_guest_throughput_benchmark`, `GuestThroughputBenchmarkResult`, disposition mapping |
| Guest throughput seam | `run_datapath_guest_throughput_cpu_seam`, `DatapathGuestThroughputCpuSeamOutcome` |
| Gate D guest-throughput | `GateDDatapathGuestThroughputResult`, guest-execution → benchmark → throughput seam orchestration |
| UEFI + xtask | `datapath-guest-throughput` feature chain, serial markers, coverage pass |
| IPC ring buffer | `hv-datapath` + `guest-common` occupancy semantics |

## Phase 25 deferrals closed

| Phase 25 item | Phase 26 disposition |
|---------------|---------------------|
| In-VM 200 Mbit/s measurement | **Closed (mock default)** — guest runtime relay benchmark with official metric; target enforced in Gate D init |
| Live VMX in-VM throughput | **Partial** — throughput seam validates measurement plan against execution seam; sustained guest-side relay loop deferred |

## Issues found and fixed

| Issue | Fix |
|-------|-----|
| IPC queue treated monotonic head as capacity | Ring-buffer occupancy check (`head - tail >= queue_slots`) and `% queue_slots` slot indexing in `hv-datapath` (aligned with guest-common) |
| Guest throughput benchmark reset queues per run | Reverted; sustained benchmark reuses one runtime — MID drain keeps producer unblocked |
| Throughput seam re-VMLAUNCHed after execution seam | Throughput seam now takes `DatapathGuestExecutionCpuSeamOutcome` and validates measurement plan only |
| `Executed` disposition mapped from VMLAUNCH without live stats | `guest_throughput_disposition_for_seam(live_measurement_completed, …)`; Gate D fails closed if `Executed` without live measurement |
| Duplicate VMCS launch-input construction in throughput init | Removed; throughput reuses execution seam outcome |
| Missing throughput validations vs Phase 25 | Added partition/vmexit/measurement-run consistency checks |
| `build_guest_live_vmcs_fields` cfg omitted throughput feature | Extended cfg to include `datapath-guest-throughput` |
| No UEFI marker for live throughput execution | Added `GATE_D_GUEST_THROUGHPUT_EXECUTED_MARKER` (conditional log) |
| Weak integration tests | Expanded boot/EFI tests with execution-seam parity and boot-info checks |
| xtask wall-clock benchmark replanned on queue full | Removed obsolete workaround after ring-buffer fix |
| `platform-contract.md` stale | Updated Phases 25–26 status |

## Feature matrix

| Feature | Crate | Default | UEFI chain | Effect |
|---------|-------|---------|------------|--------|
| `datapath-guest-throughput` | `hv-datapath` | always | n/a | Mock guest throughput benchmark + disposition helpers |
| `datapath-guest-throughput` | `hv-x86-cpu` | off | off | `run_datapath_guest_throughput_cpu_seam` (execution-context validation) |
| `datapath-guest-throughput` | `hv-hypervisor-boot` | off | off | Gate D guest-execution + in-VM mock benchmark + throughput seam |
| `datapath-guest-throughput` | `hv-hypervisor-efi` | off | opt-in | Guest-throughput boot entry + throughput markers |

## Serial markers

- `GATE_D_GUEST_THROUGHPUT_MARKER` — in-VM guest throughput orchestration succeeded
- `GATE_D_GUEST_THROUGHPUT_TARGET_MET_MARKER` — mock benchmark minimum run met 200 Mbit/s target (validate-only default)
- `GATE_D_GUEST_THROUGHPUT_EXECUTED_MARKER` — live in-VM throughput measured under VMX (ring-0 firmware only, deferred)
- Inherited markers from Phase 25 (guest execution, boot-info, source ELF, etc.)

## Verification

- `cargo xtask build-guests`
- `cargo test -p hv-datapath guest_throughput`
- `cargo test -p hv-x86-cpu --features datapath-guest-throughput`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-throughput`
- `cargo test -p hv-hypervisor-efi --features datapath-guest-throughput`
- `cargo clippy -p hv-hypervisor-boot -p hv-hypervisor-efi -p hv-x86-cpu -p hv-datapath --features datapath-guest-throughput -- -D warnings`

## Review status

Phase 26 closes the in-VM throughput scaffolding gap from Phase 25: Gate D runs the guest runtime relay benchmark with the official 200 Mbit/s metric, validates the target in init, and wires a throughput CPU seam that builds on the guest execution seam without double VMLAUNCH. Host/CI tests remain validate-only with mock timing; `GuestThroughputDisposition::Executed` is reserved for future live in-VM measurement. Sustained in-guest benchmark loops in freestanding guest source remain deferred.
