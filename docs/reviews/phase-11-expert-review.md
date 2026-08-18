# Phase 11 expert review

Multi-domain review of UEFI Gate C closure: embedded layout snapshot, firmware Gate C orchestration, and snapshot cross-validation (`cursor/phase-11-uefi-gate-c-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Layout snapshot ABI | `LayoutSnapshot`, guest/IPC regions, PCI assignments, reserve binding |
| Snapshot conversion | `layout_snapshot_from_platform_ir()`, `static_platform_ir_from_layout_snapshot()` |
| Gate C firmware path | `boot_from_transfer_and_init_gate_c_from_snapshots()` on UEFI hypervisor entry |
| Embedded config generation | `LAYOUT_SNAPSHOT` in `build/hypervisor_embedded_config.rs` via `hv-config` |
| Requirements/layout binding | Reserve phys/size must match between requirements and layout snapshots |
| UEFI hypervisor entry | `boot_hypervisor_from_transfer()` now runs Gate C (VMX + EPT + VT-d mock inits) |
| Build / CI | OVMF smoke unchanged (TCG-friendly `configs/ovmf-smoke.yaml`); Gate C on firmware path |

## Phase 10 deferrals closed

| Phase 10 item | Phase 11 disposition |
|---------------|---------------------|
| UEFI Gate C path (Phase 10 #9) | **Closed** — `LayoutSnapshot` embedded in hypervisor image; firmware calls `boot_from_transfer_and_init_gate_c_from_snapshots()`. |
| Static layout dependency on host only (Phase 10 gate_c risk) | **Closed** — compile-time layout serialized into fixed-size snapshot at image build time. |
| Layout/requirements reserve mismatch undetected (Phase 10 static layout risk) | **Closed** — `static_platform_ir_from_layout_snapshot()` rejects reserve phys/size mismatch with requirements snapshot. |

## Code coverage

Fresh run: `cargo xtask coverage` (2026-08-18).

| Metric | Value |
|--------|-------|
| Workspace line coverage | **95.14%** (7553 lines, 367 missed) |
| Minimum threshold | 95% |
| Result | **pass** |

### Phase 11 file line coverage (selected)

| File | Lines | Missed | Cover |
|------|------:|-------:|------:|
| `hv-boot-abi/src/layout_snapshot.rs` | 15 | 0 | 100.00% |
| `hv-hypervisor-boot/src/snapshot.rs` | 297+ | partial | ≥96% (layout conversion paths covered) |
| `hv-hypervisor-boot/src/gate_c.rs` | 107+ | partial | ≥93% |
| `hv-hypervisor-efi/src/lib.rs` | 125+ | 0 | 100.00% |

## Domain expert notes

### Layout snapshot ABI (`hv-boot-abi`)

- **Finding:** Gate C on firmware cannot use host-only `StaticPlatformIR` with heap strings; EPT/VT-d planning needs only resolved addresses and PCI assignments.
- **Fix:** Added `LayoutSnapshot` with bounded guest/IPC region arrays (`MAX_LAYOUT_GUEST_REGIONS`, `MAX_LAYOUT_IPC_REGIONS`) and PCI assignments (`MAX_LAYOUT_PCI_DEVICES`). Reserve phys/size duplicated for cross-validation against `RequirementsSnapshot`.
- **Risk (deferred):** Snapshot omits partition/channel/kind strings; Gate C planning does not need them today but future datapath work may require compact identifiers.

### Snapshot conversion (`hv-hypervisor-boot`)

- **Finding:** Firmware and host paths must share one reconstruction path from embedded snapshots to `StaticPlatformIR` for Gate C orchestration.
- **Fix:** `layout_snapshot_from_platform_ir()` serializes planned layout; `static_platform_ir_from_layout_snapshot()` rebuilds planning IR with empty string placeholders and validates reserve binding against requirements snapshot.
- **Tests:** Round-trip preserves guest/IPC/PCI planning fields; reserve mismatch rejected.

### Gate C firmware orchestration

- **Finding:** Phase 10 Gate C was host-only because firmware lacked layout metadata.
- **Fix:** `boot_from_transfer_and_init_gate_c_from_snapshots()` chains requirements restore, layout restore, Gate B validation, and VMX/EPT/VT-d mock inits. `hv-hypervisor-efi::boot_hypervisor_from_transfer()` delegates to this entry with both embedded snapshots.
- **Risk (deferred):** Backend failure mapping through orchestrated Gate C entry still not end-to-end tested; crate-level mock backend tests suffice for Phase 11.

### Embedded config generation (`hv-config`)

- **Finding:** Hypervisor UEFI build copies pre-generated `hypervisor_embedded_config.rs`; Phase 10 embedded only requirements snapshot + reserve fields.
- **Fix:** `render_hypervisor_embedded_config()` now emits `LAYOUT_SNAPSHOT` alongside `REQUIREMENTS_SNAPSHOT` and `CONFIG_DIGEST`. `cargo xtask build-boot-chain` regenerates artifacts under `build/` before UEFI compile.
- **Risk (deferred):** Layout snapshot is not independently digest-sealed; binding relies on co-generation from the same config compile and reserve cross-check at runtime.

### UEFI hypervisor entry

- **Finding:** Phase 9–10 firmware stopped at Gate B (`boot_from_transfer_and_init_vmx()`).
- **Fix:** `hv-hypervisor-efi-bin` passes `LAYOUT_SNAPSHOT` into `boot_hypervisor_from_transfer()`; success log line is `hypervisor Gate C boot succeeded`.
- **Risk (deferred):** OVMF smoke still uses `configs/ovmf-smoke.yaml` with optional VMX/EPT/VT-d; production `configs/qemu.yaml` Gate C on KVM OVMF not exercised in CI.

### Build / CI / OVMF smoke

- **Finding:** Smoke validates chain-load without `Aborted`; Gate C adds mock EPT/VT-d init on optional-feature smoke config.
- **Fix:** Smoke boot chain rebuild includes layout snapshot; serial evaluation unchanged (boot attempt, no failure marker).
- **Risk (deferred):** No serial log assertion for `hypervisor Gate C boot succeeded` yet; smoke remains chain-load focused.

## Findings and disposition

### MUST FIX (applied)

1. **Layout snapshot ABI** — `hv-boot-abi::LayoutSnapshot` with bounded guest/IPC/PCI arrays and reserve fields.
2. **Snapshot conversion** — `layout_snapshot_from_platform_ir()` and `static_platform_ir_from_layout_snapshot()` with reserve cross-validation.
3. **Gate C firmware entry** — `boot_from_transfer_and_init_gate_c_from_snapshots()` and UEFI hypervisor delegation.
4. **Embedded config** — `LAYOUT_SNAPSHOT` generated into `hypervisor_embedded_config.rs`.

### SHOULD FIX (applied)

5. **Coverage and tests** — Layout round-trip, reserve mismatch, Gate C from snapshots, hypervisor-efi Gate C path; workspace line coverage **95.14%**.
6. **Documentation** — Architecture, platform contract, OVMF boot, README updated for Phase 11 UEFI Gate C closure.

### Documented (deferred)

7. **Hardware VMXON/EPT paging / VT-d IOMMU** — Mock backends only; x86 hardware programming remains future work.
8. **Layout snapshot digest seal** — Co-generated with requirements snapshot; no independent integrity tag.
9. **Production OVMF Gate C smoke** — Host-tested with `configs/qemu.yaml`; KVM OVMF with required features deferred.
10. **e1000 datapath (Gate D)** — Unchanged.

## Delivered

| Component | Role |
|-----------|------|
| `hv-boot-abi::layout_snapshot` | Fixed-size layout snapshot for firmware Gate C |
| `hv-hypervisor-boot::snapshot` | Layout snapshot encode/decode + reserve binding |
| `hv-hypervisor-boot::gate_c` | `boot_from_transfer_and_init_gate_c_from_snapshots()` |
| `hv-hypervisor-efi` | UEFI entry runs Gate C with embedded snapshots |
| `tools/hv-config` | Emits `LAYOUT_SNAPSHOT` in hypervisor embedded config |

## Verification

- `cargo test --workspace` — pass
- `cargo clippy --all-targets --all-features -- -D warnings` — pass
- `cargo xtask coverage` — pass (**95.14%** line coverage)
- `cargo xtask build-boot-chain` — produces `build/boot-chain/hv-loader.efi` and `hv-hypervisor.efi`
- `cargo xtask ovmf-smoke-boot` — pass
- `cargo clippy --release --target x86_64-unknown-uefi --manifest-path crates/hv-loader-efi-bin/Cargo.toml -- -D warnings` — pass
- `cargo clippy --release --target x86_64-unknown-uefi --manifest-path crates/hv-hypervisor-efi-bin/Cargo.toml -- -D warnings` — pass

## Review status

All MUST and SHOULD items above are applied. Phase 10 deferrals for UEFI Gate C and layout embedding are closed. Remaining deferred items are documented with explicit phase ownership. PR **#12** is ready for human review.
