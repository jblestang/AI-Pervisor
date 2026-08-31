# Phase 29 expert review (virtualization)

Multi-domain review of in-VM guest relay frame measurement: boot-info counter tails, throughput seam `Executed` disposition, and KVM measurement smoke tier (`cursor/phase-29-guest-relay-measurement-0b4f`).

**Reviewer lens:** x86 VMX bring-up, EPT/identity mapping, guest lifecycle, and attestation boundaries.

## Executive summary

Phase 29 is a **sound scaffolding step** that closes Phase 28 deferrals without over-claiming full in-VM proof. The fail-closed disposition chain from Phase 27 is preserved: `GuestThroughputDisposition::Executed` now requires non-zero in-VM relay frame counts read after the execution seam reports `Executed`, plus the existing host-side sustained relay validation.

What Phase 29 **does** prove (when REAL_HW succeeds on bare metal):

- Guest boot-info blobs carry an 8-byte relay counter tail.
- Freestanding guest firmware increments that counter during sustained relay loops.
- Gate D reads counters post-VMLAUNCH and gates the throughput `Executed` seam on `min(frames) ≥ 64`.
- KVM smoke can require both hypervisor `GATE_D_GUEST_THROUGHPUT_EXECUTED` and guest serial completion markers.

What Phase 29 **does not** yet prove:

- That guests actually ran to completion under a correct VM-exit / VMRESUME loop.
- End-to-end pipeline frame delivery (in→mid→out as one logical frame stream).
- Attested or hypervisor-verified frame counts (counters are guest-writable).
- Wall-clock in-VM throughput (timing remains mock-derived).

**Verdict:** Approve as incremental Gate D measurement wiring. Treat `Executed` as “VMLAUNCH batch succeeded and guest-reported counters meet threshold under identity mapping,” not as cryptographic attestation of datapath throughput.

---

## Domains reviewed

| Domain | Scope | Virtualization notes |
|--------|--------|----------------------|
| Guest ABI | `GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES`, tail offset helper | Tail appended without ABI version bump; layout parser tolerates trailing bytes |
| Guest firmware | `guest-common::record_relay_frame_completed`, sustained loops | Guest-writable counter; no atomics (single vCPU assumed) |
| Host install | `install_guest_elf_with_boot_info`, RDI patch | Boot info colocated with ELF; GPA used as HPA (identity contract) |
| CPU execution seam | `execute_datapath_guest_vmlaunch_fields_if_enabled` | VMLAUNCH-only; `_host_exit_phys` ignored in launch loop |
| CPU measurement seam | `measure_in_vm_relay_frames_from_boot_infos` | Raw HPA dereference; min across partitions |
| Gate D wiring | Measurement sites from partition launches + boot-info blobs | Fail-closed cross-checks on frame counts and disposition |
| KVM smoke | Measurement tier on executed + guest relay-complete markers | Environment-dependent; nested KVM / no-EPT hosts will not reach tier |

---

## Phase 28 deferrals closed

| Phase 28 item | Phase 29 disposition |
|---------------|---------------------|
| In-VM relay frame measurement | **Closed (scaffolding)** — boot-info counter tail + hypervisor read after VMLAUNCH batch |
| Smoke `GATE_D_GUEST_THROUGHPUT_EXECUTED` | **Closed** — strict measurement evaluator when executed marker present in serial log |
| Guest relay-complete marker in smoke | **Closed** — `GUEST_DATAPATH_RELAY_BENCHMARK_COMPLETE_MARKER` required for measurement tier |

---

## Virtualization analysis

### 1. VMX guest lifecycle — highest-risk gap

The execution seam still performs **VMLAUNCH per partition without a VM-exit dispatch loop**. In `execute_datapath_guest_vmlaunch_fields_if_enabled`, `host_exit_phys` is validated at seam entry but **discarded** in the launch loop (`_host_exit_phys`).

Intel semantics: a successful VMLAUNCH enters guest mode; host code after `vmlaunch` runs only after a VM-exit whose handler eventually returns control to the host RIP programmed in the VMCS. Without wiring the exit stub and VMRESUME, one of the following must be true for counters to advance and markers to appear:

- VMCS host-state fields cause exits to land back in hypervisor text that can fall through (unlikely for 64-frame relay + serial output), or
- VMLAUNCH returns `ExecutionFailed` / `SeamValidated` and counters stay at zero (fail-closed — current CI path), or
- Platform-specific behaviour masks the gap (e.g. immediate fault exit to a lucky RIP).

**Implication:** On bare metal, reaching measurement tier is the real integration test for guest run-to-completion. Unit tests correctly stay validate-only. **Phase 30 should implement VM-exit handling and VMRESUME** before treating relay measurement as production-grade.

### 2. Address-space contract — identity HPA read

Measurement reads use:

```rust
let host_ptr = site.host_boot_info_phys as *const u8;
read_relay_frames_completed_from_boot_info_host(host_ptr, site.boot_info_size)
```

`boot_info_guest_phys` from partition records is the same address patched into guest RDI and passed as `host_boot_info_phys`. That is consistent with the project’s **identity bring-up** contract (`hv-vmx` launch plan: guest entry GPA equals host allocation; flat EPT).

**Risk:** When EPT maps guest RAM at non-identity HPAs, this read path will silently read wrong memory or fault. A correct long-term approach is GPA→HPA translation via EPT walk, or placing the counter in a hypervisor-owned shared page mapped read-only to the guest.

### 3. Measurement semantics — per-partition min, not pipeline E2E

Each partition (in/mid/out) runs its **own** 64-iteration sustained loop and increments **its own** boot-info counter. Gate D takes the **minimum** across three sites.

| Interpretation | Valid? |
|----------------|--------|
| “Each guest vCPU completed ≥64 local relay iterations” | Yes, if guests actually ran |
| “64 synthetic frames traversed in→mid→out end-to-end” | **No** — host `validate_sustained_host_relay_benchmark` validates E2E on the host runtime path; in-VM counters are per-partition loop counts |

The min aggregation is **conservative** (straggler partition blocks `Executed`) but can **over-report** relative to true pipeline frames if a guest increments without participating in IPC (malicious or buggy guest).

### 4. Trust model — guest-self-reported counters

The tail counter lives in guest RAM the guest can write via `record_relay_frame_completed`. There is no:

- Hypervisor-only mapping (guest read-only),
- Cross-check against IPC queue head/tail or doorbell MMIO,
- Signature or monotonic sequence validated by the host.

For Gate D **smoke / benchmark scaffolding**, this is acceptable. For **security or SLA attestation**, counters must move to hypervisor-visible shared state or be derived from VM-exits on IPC/MMIO.

### 5. Fail-closed disposition chain — preserved (good)

Phase 27’s fix remains intact:

1. Execution seam must be `Executed` before measurement reads non-zero (`measure_in_vm_relay_frames_from_boot_infos` returns 0 otherwise).
2. Throughput seam `Executed` requires `in_vm_relay_frames >= expected_relay_frames` when `datapath-guest-relay-measurement` is enabled.
3. `guest_throughput_result_with_live_relay` requires both execution executed and frame threshold for `GuestThroughputDisposition::Executed`.
4. Gate D rejects `Executed` throughput if `in_vm_relay_frames < expected_relay_frames`.
5. UEFI logs `GATE_D_GUEST_THROUGHPUT_EXECUTED_MARKER` only when disposition is `Executed`.

Host sustained relay validation remains **orthogonal** — it does not upgrade disposition alone.

### 6. ABI tail without version bump

`GUEST_ABI_VERSION` stays at 1 while `header.size` now includes 8 tail bytes. `GuestBootInfoView::parse` allows `device_end ≤ header.size`, so the tail is tolerated but **not explicitly validated** (no check for `GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES`).

| Scenario | Outcome |
|----------|---------|
| New hypervisor + new guests | Works — builder zero-initializes tail |
| New guests + old hypervisor that ignores tail | Guest still increments; hypervisor may not read |
| Mismatched `header.size` without tail | Parse/measurement offset errors — fail-closed |

Recommend **`GUEST_ABI_VERSION = 2`** (or a dedicated extension flag) when measurement becomes mandatory.

### 7. Throughput timing — still synthetic

`apply_live_guest_throughput_benchmark` computes Mbit/s from `in_vm_relay_frames * mock_nanos_per_frame`. In-VM **wall time** is not measured. Throughput numbers remain **plan validation**, not benchmark results.

### 8. `live_relay_validated` naming drift

`DatapathGuestThroughputCpuSeamOutcome.live_relay_validated` is still set from `execution_seam.disposition == Executed`, not from successful in-VM frame measurement. Gate D adds separate checks when measurement is enabled, but the field name overstates what was validated.

### 9. KVM smoke measurement tier

`evaluate_live_qemu_smoke_serial` upgrades to measurement tier when the serial log contains `GATE_D_GUEST_THROUGHPUT_EXECUTED_MARKER`, then requires guest `GUEST_DATAPATH_RELAY_BENCHMARK_COMPLETE_MARKER`.

| Environment | Expected outcome |
|-------------|------------------|
| Bare metal + VMX + EPT + 8 GiB | May reach measurement tier if guests run |
| OVH classic VPS (no nested EPT / `ept` absent) | Platform validation fails before executed markers |
| CI / `cargo test` | Validate-only; no false `Executed` |

Smoke tier is **string-based** — appropriate for boot-chain regression, not a substitute for host-side counter verification in the evaluator.

---

## Issues found (Phase 29 review)

| Severity | Issue | Recommendation |
|----------|-------|----------------|
| **High** | No VM-exit / VMRESUME loop; exit stub unused in launch batch | Phase 30: exit dispatcher + resume; only then treat in-VM counters as execution proof |
| **High** | Measurement reads GPA as HPA via cast | Document identity contract; add EPT-aware read or shared hypervisor page |
| **Medium** | Per-partition counters ≠ E2E pipeline frames | Document semantics; consider out-partition-only or IPC-derived count for E2E |
| **Medium** | Guest-writable attestation surface | Accept for smoke; harden before security claims |
| **Medium** | ABI tail without version bump | Bump version or add extension when stabilizing |
| **Low** | Mock throughput timing | TSC / VM-exit-based timing in later phase |
| **Low** | `live_relay_validated` misnamed vs measurement | Rename or tie to `in_vm_relay_frames >= expected` |
| **Low** | No live integration test with forced counter values under REAL_HW | Add harness test when VM-exit loop exists |

No blocking correctness bugs found in the **fail-closed validate-only CI path**. Issues above affect **claims** about live in-VM execution, not compilation or unit-test integrity.

---

## Feature matrix

| Feature | Crate | Default | UEFI chain | Effect |
|---------|-------|---------|------------|--------|
| `datapath-guest-relay-measurement` | `hv-x86-cpu` | off | off | Read in-VM relay frames; throughput seam `Executed` when frames ≥ 64 |
| `datapath-guest-relay-measurement` | `hv-hypervisor-boot` | off | off | Gate D measurement sites + disposition wiring |
| `datapath-guest-relay-measurement` | `hv-hypervisor-efi` | off | opt-in | Same relay-live boot entry with measurement enabled |
| `build-boot-chain-live` | xtask | n/a | n/a | Builds hypervisor with measurement feature |

---

## Serial markers (measurement tier)

- All Phase 28 validate-only markers
- `Gate D: guest throughput measured under live VMX` (`GATE_D_GUEST_THROUGHPUT_EXECUTED`)
- `GUEST: datapath relay benchmark complete` (guest firmware, at least once)

---

## Verification

- `cargo test -p hv-guest-abi`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-relay-measurement`
- `cargo test -p xtask evaluate_gate_d`
- `cargo xtask build-guests && cargo xtask build-boot-chain-live`
- Bare metal: `cargo xtask live-qemu-smoke` (measurement tier if executed + guest markers present)
- VPS: `cargo xtask ovmf-smoke-boot` only (no REAL_HW measurement expectation)

---

## Review status

Phase 29 closes Phase 28 deferrals for **in-VM relay frame measurement scaffolding**. The design correctly extends Phase 27’s fail-closed model: host proxy relay frames cannot alone produce `Executed`; guest boot-info counters are read and min-aggregated after the execution seam. Full virtualization correctness — guest run-to-completion under VMX, EPT-safe reads, E2E frame attestation, and real throughput timing — remains **Phase 30+** work.
