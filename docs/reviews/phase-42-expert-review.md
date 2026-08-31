# Phase 42 expert review

Addresses Phase 41 expert review findings (`cursor/phase-42-hypervisor-tsc-review-fixes-0b4f`).

## Fixes applied

| Review finding | Fix |
|----------------|-----|
| Stale guest-TSC doc on `InVmRelayMeasurement.elapsed_tsc` | Document hypervisor execution bracket |
| Inverted TSC brackets accepted silently | `validate_hypervisor_tsc_bracket()` fails closed at publish |
| TSC sampled before live execution gate | Bracket start/end only inside `live_execution_environment_ready()` |
| No cross-check after publish | Verify measurement page TSC matches execution seam bracket |
| Gate D missing bracket validation | Reject inverted hypervisor TSC when execution is `Executed` |

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-relay-measurement`
- `cargo xtask build-guests && cargo xtask build-boot-chain-live`
