# Phase 34 expert review

Addresses Phase 33 expert review findings (`cursor/phase-34-measurement-page-review-fixes-0b4f`).

## Fixes applied

| Review finding | Fix |
|----------------|-----|
| `parse_extension_bytes` duplicated ABI logic, skipped version checks | Removed local parser; shared `parse_relay_measurement_page_header()` in `hv-guest-abi` with magic/version/GPA validation |
| Measurement seam fell back to boot-info EPT when no host page | `measure_in_vm_relay_from_context` requires `measurement_page_host_phys` on `Executed`; authoritative read from hypervisor page only |
| Gate D validated foundation boot-info blobs, not patched installed blobs | Install path validates patched blob GPA before install; post-execution EPT read cross-checks installed boot info `measurement_page_gpa` |
| EPT append did not refresh installed host tables | After `append_ept_guest_mapping`, Gate D calls `install_ept_tables()` to rebind host copy |
| EPT root table still single-entry stub for high GPA | Documented/deferred — runtime append records mapping metadata; full root-table walk for `0xFEB2_0000` remains follow-up |

## Verification

- `cargo test -p hv-guest-abi -p hv-ept -p hv-guest-boot`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-relay-measurement`
- `cargo xtask build-guests && cargo xtask build-boot-chain-live`
