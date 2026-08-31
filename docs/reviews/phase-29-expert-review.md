# Phase 29 expert review

Multi-domain review of in-VM guest relay frame measurement: boot-info counter tails, throughput seam `Executed` disposition, and KVM measurement smoke tier (`cursor/phase-29-guest-relay-measurement-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Guest ABI | `GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES`, relay counter tail appended to boot info blobs |
| Guest firmware | `guest-common` increments counter each sustained relay iteration |
| CPU measurement seam | `measure_in_vm_relay_frames_from_boot_infos`, `in_vm_relay_frames` on throughput outcome |
| Gate D wiring | Reads boot-info counters after guest execution; fail-closed `Executed` requires frame counts |
| KVM smoke | Measurement tier accepts `GATE_D_GUEST_THROUGHPUT_EXECUTED` + guest relay-complete marker |

## Phase 28 deferrals closed

| Phase 28 item | Phase 29 disposition |
|---------------|---------------------|
| In-VM relay frame measurement | **Closed** — boot-info counter tail + hypervisor read after VMLAUNCH |
| Smoke `GATE_D_GUEST_THROUGHPUT_EXECUTED` | **Closed** — strict measurement evaluator when executed marker present |
| Guest relay-complete marker in smoke | **Closed** — `GUEST_DATAPATH_RELAY_BENCHMARK_COMPLETE_MARKER` required for measurement tier |

## Feature matrix

| Feature | Crate | Default | UEFI chain | Effect |
|---------|-------|---------|------------|--------|
| `datapath-guest-relay-measurement` | `hv-x86-cpu` | off | off | Read in-VM relay frames; throughput seam `Executed` when frames ≥ 64 |
| `datapath-guest-relay-measurement` | `hv-hypervisor-boot` | off | off | Gate D measurement sites + disposition wiring |
| `datapath-guest-relay-measurement` | `hv-hypervisor-efi` | off | opt-in | Same relay-live boot entry with measurement enabled |
| `build-boot-chain-live` | xtask | n/a | n/a | Builds hypervisor with measurement feature |

## Serial markers (measurement tier)

- All Phase 28 validate-only markers
- `Gate D: guest throughput measured under live VMX` (`GATE_D_GUEST_THROUGHPUT_EXECUTED`)
- `GUEST: datapath relay benchmark complete` (guest firmware, at least once)

## Verification

- `cargo test -p hv-guest-abi`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-relay-measurement`
- `cargo test -p xtask evaluate_gate_d`
- `cargo xtask build-guests && cargo xtask build-boot-chain-live`

## Review status

Phase 29 closes Phase 28 deferrals for in-VM relay frame measurement scaffolding. Guests increment a boot-info counter tail; Gate D reads counters after live guest execution and upgrades throughput disposition to `Executed` only when all partitions report ≥ 64 frames. Full VM-exit/resume guest run-to-completion under nested KVM remains environment-dependent; measurement smoke tier validates executed markers when REAL_HW completes multi-partition relay.
