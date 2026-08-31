# Phase 27 expert review

Multi-domain review of Gate D sustained guest relay live wiring: freestanding guest firmware relay loops plus host live throughput integration (`cursor/phase-27-guest-relay-live-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Guest firmware | `guest-common` sustained in/mid/out relay loops, `GUEST_DATAPATH_RELAY_BENCHMARK_COMPLETE_MARKER` |
| Live relay benchmark | `run_sustained_guest_relay_benchmark`, `guest_throughput_result_with_live_relay` |
| Gate D guest-relay-live | `GateDDatapathGuestRelayLiveResult`, sustained relay frames in throughput init |
| Guest throughput seam | `DatapathGuestThroughputCpuSeamOutcome.live_relay_validated` |
| UEFI + xtask | `datapath-guest-relay-live` feature chain, integration tests, live boot-chain build |

## Phase 26 deferrals closed

| Phase 26 item | Phase 27 disposition |
|---------------|---------------------|
| Sustained in-guest benchmark loops | **Closed** — `guest-common` runs 64-frame sustained relay per partition |
| Live in-VM throughput with relay stats | **Closed (host wiring)** — `guest_throughput_result_with_live_relay` upgrades disposition when VMX execution + relay frames complete |
| `GuestThroughputDisposition::Executed` without live stats | **Closed** — relay-live path applies live relay benchmark stats before disposition mapping |

## Feature matrix

| Feature | Crate | Default | UEFI chain | Effect |
|---------|-------|---------|------------|--------|
| `datapath-guest-relay-live` | `hv-datapath` | always | n/a | Sustained guest relay benchmark + live throughput wiring |
| `datapath-guest-relay-live` | `hv-x86-cpu` | off | off | `live_relay_validated` on throughput seam outcome |
| `datapath-guest-relay-live` | `hv-hypervisor-boot` | off | off | Gate D sustained relay frames + live throughput disposition |
| `datapath-guest-relay-live` | `hv-hypervisor-efi` | off | opt-in | Guest-relay-live boot entry (extends throughput markers) |

## Serial markers

- `GUEST_DATAPATH_RELAY_BENCHMARK_COMPLETE_MARKER` — guest firmware sustained relay loop completed (freestanding guests)
- Inherited Phase 26 throughput markers (`GATE_D_GUEST_THROUGHPUT_*`) — hypervisor orchestration and live execution disposition

## Verification

- `cargo xtask build-guests`
- `cargo test -p hv-datapath guest_relay_live`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-live`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-relay-live`
- `cargo test -p hv-hypervisor-efi --features datapath-guest-relay-live`
- `cargo clippy -p hv-hypervisor-boot -p hv-hypervisor-efi -p hv-x86-cpu -p hv-datapath --features datapath-guest-relay-live -- -D warnings`

## Review status

Phase 27 closes the Phase 26 deferrals for sustained guest relay loops and live throughput wiring. Freestanding guest firmware runs 64-frame in→mid→out relays; Gate D validates sustained relay frames on the host runtime path and maps live throughput disposition when VMX guest execution completes with sufficient relay frames. Host/CI tests remain validate-only; REAL_HW ring-0 firmware with live execution may reach `GuestThroughputDisposition::Executed`.
