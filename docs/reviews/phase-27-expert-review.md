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
| Host sustained relay validation | **Closed** — `validate_sustained_host_relay_benchmark` enforces 200 Mbit/s on host runtime path |
| Live in-VM throughput with relay stats | **Partial** — disposition wiring exists; `Executed` requires in-VM relay frame counts (fail-closed until execution seam reports them) |
| `GuestThroughputDisposition::Executed` from host proxy | **Closed (fixed in review)** — host relay frames no longer upgrade disposition; `in_vm_relay_frames` required |

## Issues found and fixed (Phase 27 review)

| Issue | Fix |
|-------|-----|
| Host sustained relay frames used as in-VM live measurement proof | Split host validation (`validate_sustained_host_relay_benchmark`) from in-VM disposition (`in_vm_relay_frames` parameter); Gate D passes 0 until execution seam reports counts |
| Host sustained relay target not enforced when live_completed false | `validate_sustained_host_relay_benchmark` always checks 200 Mbit/s target on host path |
| False `Executed` on REAL_HW from VMLAUNCH + host relay | Fail-closed: `Executed` requires `in_vm_relay_frames >= GUEST_RELAY_BENCHMARK_FRAMES` |
| Weak unit test implied host relay proved Executed | Tests now cover validate-only with execution=true/in_vm=0 and Executed only with in_vm frames |
| `platform-contract.md` stale | Updated Phases 18–27 status |

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

Phase 27 closes the Phase 26 deferrals for sustained guest relay loops and host sustained relay validation. Freestanding guest firmware runs 64-frame in→mid→out relays; Gate D validates sustained relay frames and throughput target on the host runtime path. `GuestThroughputDisposition::Executed` remains fail-closed until the execution seam reports in-VM relay frame counts; REAL_HW ring-0 firmware may reach `Executed` once that measurement path lands.
