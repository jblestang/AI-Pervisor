# Phase 44 expert review

Addresses Phase 43 expert review findings (`cursor/phase-44-vmexit-frame-counter-review-fixes-0b4f`).

## Fixes applied

| Review finding | Fix |
|----------------|-----|
| `vmlaunch_to_host` / `vmresume_to_host` resumed at stub-install loop head | Push explicit resume label; VM-exit returns to host dispatch instead of re-VMLAUNCH |
| Measurement page reset errors ignored | Propagate `reset_relay_measurement_page_frames()` failures |
| Guest execution silently skipped VM-exit counting | Gate D requires installed relay measurement page host phys |
| Loop counter could diverge from page counter | Cross-check page frames vs dispatch loop; prefer page count |
| Unhandled EPT violations could VMRESUME forever | Fail closed on unexpected EPT violations during out-partition relay run |
| Zero VM-exit frames reached publish path | Reject zero `vmexit_relay_frames` before publish on `Executed` |

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-relay-measurement`
- `cargo xtask build-guests && cargo xtask build-boot-chain-live`
