# Phase 41 expert review

Hypervisor-derived TSC for relay measurement publish (`cursor/phase-41-hypervisor-derived-tsc-0b4f`).

## Changes

| Component | Change |
|-----------|--------|
| Host TSC | `read_timestamp_counter()` / live `RDTSC` gated like other privileged instructions |
| Guest execution seam | Brackets live VMLAUNCH batch with `hypervisor_tsc_start` / `hypervisor_tsc_end` |
| Publish | `publish_relay_measurement_page_authoritative()` writes host TSC bracket, not guest boot-info TSC |
| Measurement | `elapsed_tsc` derived from hypervisor bracket; rejects zero elapsed on `Executed` |
| Guest firmware | Out partition no longer records guest RDTSC in boot-info tail |

## Trust model

Frame counts remain IPC-derived at publish time. Elapsed timing now originates from hypervisor RDTSC brackets around guest execution instead of guest-writable boot-info TSC fields.

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p hv-ept`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-relay-measurement`
- `cargo xtask build-guests && cargo xtask build-boot-chain-live`
