# Phase 38 expert review

Addresses Phase 37 expert review findings (`cursor/phase-38-readonly-measurement-review-fixes-0b4f`).

## Fixes applied

| Review finding | Fix |
|----------------|-----|
| Guest boot-info IPC cross-check bypassed after publish | Restore `guest_boot_info_frames` in measurement; `end_to_end_relay_frames` uses guest tail vs IPC |
| Publish skipped extension validation | Validate magic, version, and non-zero `measurement_page_gpa` before publish |
| No post-publish host page verification | Read back host page and verify published frame count |
| Read-only mapping only checked on materialized leaf | Gate D also verifies installed `EptProgrammedMapping.guest_writable == false` |
| Guest over-reporting boot-info frames undetected | Reject when guest boot-info frames exceed IPC delivered tail |

## Verification

- `cargo test -p hv-ept`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-relay-measurement`
- `cargo xtask build-guests && cargo xtask build-boot-chain-live`
