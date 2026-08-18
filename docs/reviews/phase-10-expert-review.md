# Phase 10 expert review

Multi-domain review of Gate C foundation: EPT/VT-d init planning, mock backends, and host-tested Gate C orchestration (`cursor/phase-10-gate-c-foundation-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| EPT foundation | `hv-ept` init plan, identity mappings, mock backend, feature-gated init |
| VT-d foundation | `hv-vtd` init plan, PCI device assignments, mock backend, interrupt remapping flag |
| VMX chain integration | Gate C reuses Phase 9 `plan_vmx_init()` / `init_vmx()` before EPT/VT-d |
| Gate C orchestration | `hv-hypervisor-boot::gate_c` chains VMX + EPT + VT-d mock inits after Gate B validation |
| Static layout dependency | Gate C requires compile-time `StaticPlatformIR`; not embedded in UEFI hypervisor yet |
| Host boot path | `boot_from_transfer_and_init_gate_c()` host-tested with `configs/qemu.yaml` |
| UEFI hypervisor entry | Unchanged from Phase 9 — still Gate B (`boot_from_transfer_and_init_vmx`) until layout is embedded |
| Coverage / tests | Workspace line coverage ≥ 95%; Phase 10 crate coverage table below |
| Build / CI | Workspace members `hv-ept`, `hv-vtd`; OVMF smoke unchanged (TCG-friendly config) |

## Phase 9 deferrals closed

| Phase 9 item | Phase 10 disposition |
|--------------|---------------------|
| Real VMXON/EPT/VT-d enablement (Phase 9 #11) | **Partially closed** — EPT/VT-d planning seams and mock backends added; hardware VMXON/EPT paging/IOMMU programming remain deferred. |
| VT-d / interrupt remapping enablement (Phase 9 #12) | **Partially closed** — `plan_vtd_init()` + `init_vtd()` validate capability and record assignments; no IOMMU register programming yet. |

## Code coverage

Fresh run: `cargo xtask coverage` (2026-08-18).

| Metric | Value |
|--------|-------|
| Workspace line coverage | **95.31%** (7200 lines, 338 missed) |
| Minimum threshold | 95% |
| Result | **pass** |

### Phase 10 crate line coverage

| File | Lines | Missed | Cover |
|------|------:|-------:|------:|
| `hv-ept/src/backend.rs` | 11 | 0 | 100.00% |
| `hv-ept/src/error.rs` | 15 | 0 | 100.00% |
| `hv-ept/src/init.rs` | 58 | 0 | 100.00% |
| `hv-ept/src/plan.rs` | 83 | 2 | 97.59% |
| `hv-vtd/src/backend.rs` | 11 | 0 | 100.00% |
| `hv-vtd/src/error.rs` | 15 | 0 | 100.00% |
| `hv-vtd/src/init.rs` | 74 | 0 | 100.00% |
| `hv-vtd/src/plan.rs` | 32 | 0 | 100.00% |
| `hv-hypervisor-boot/src/gate_c.rs` | 107 | 7 | 93.46% |

### Coverage gaps (accepted)

- **`gate_c.rs` (93.46%)** — `map_vmx_error` / `map_vtd_error` paths for backend failures during orchestrated init are not exercised end-to-end through `boot_*_gate_c()`; individual crate tests cover backend rejection. Acceptable for mock-only Phase 10.
- **`hv-ept/src/plan.rs` (97.59%)** — `checked_add` overflow branches for EPT root table placement are defensive; reference layout cannot trigger them today.
- **No Gate C firmware path** — UEFI hypervisor does not call `gate_c`; corresponding integration coverage is intentionally host-only.

## Domain expert notes

### EPT foundation (`hv-ept`)

- **Finding:** Gate C requires EPT hierarchy planning before hardware page-table programming; identity mappings must cover guest private and IPC regions from `StaticPlatformIR`.
- **Fix:** `plan_ept_init()` builds identity mappings with page-alignment checks; EPT root table is placed at `vmx_region + VMXON_REGION_MIN_BYTES` inside the hypervisor reserve. `MockEptBackend` records plan execution; `init_ept()` gates on validated EPT observation.
- **Tests:** Unit tests in `plan.rs` / `init.rs`; integration coverage in `tests/coverage.rs` (alignment, zero-size, undersized reserve, error display, `ept_init_required`).
- **Risk (deferred):** No EPT paging structures, walk length, or memory-type attributes — mock backend only until hardware Gate C.

### VT-d foundation (`hv-vtd`)

- **Finding:** VT-d init must bind PCI BDFs to partition VM IDs and honor interrupt-remapping requirements from `PlatformRequirements`.
- **Fix:** `plan_vtd_init()` copies `layout.pci_devices` into `VtdDeviceAssignment`; `interrupt_remapping` follows `FeatureRequirement::Required | Preferred`. `MockVtdBackend` records assignments; `init_vtd()` gates on validated VT-d and interrupt-remapping observation.
- **Tests:** Unit tests cover reference PCI topology and empty `ovmf-smoke.yaml` topology; `tests/coverage.rs` verifies vm_id preservation, error display, and `vtd_init_required`.
- **Risk (deferred):** No DMAR parsing, root/context tables, or posted-interrupt programming.

### VMX chain integration

- **Finding:** Gate C must not duplicate VMX planning; EPT root table placement depends on VMXON region sizing from `VmxInitPlan`.
- **Fix:** `init_gate_c_from_validated()` plans VMX first, passes `vmx_plan` into `plan_ept_init()`, then runs `init_vmx_if_required()` before EPT/VT-d backends. Ordering matches hardware dependency (VMXON region precedes EPT structures in hypervisor reserve).
- **Risk (deferred):** Reserve layout is a compile-time contract; runtime discovery of actual firmware-reserved memory is not validated against the plan.

### Gate C orchestration (`hv-hypervisor-boot`)

- **Finding:** Phase 9 orchestration covered VMX only; Gate C must chain EPT and VT-d init behind the same fail-closed validation without pulling hardware code into firmware yet.
- **Fix:** `boot_from_transfer_and_init_gate_c()` and `boot_check_and_init_gate_c()` run Gate B validation, plan VMX/EPT/VT-d, and invoke mock backends when features are required. `GateCInitResult` exposes all three plans plus validated platform state.
- **Tests:** `boot_from_transfer_and_init_gate_c_accepts_reference_transfer`, `boot_check_and_init_gate_c_accepts_reference_inputs`, EPT planning failure mapping, optional EPT/VT-d skip when snapshot marks features optional.
- **Risk (deferred):** Gate C entry requires `&StaticPlatformIR`, which is not embedded in the UEFI hypervisor image today; firmware path stays on Gate B until Phase 11+ layout embedding.

### Static layout dependency

- **Finding:** EPT identity mappings and VT-d PCI assignments require resolved addresses and vm_ids from `StaticPlatformIR`, available only on the host compile path via `plan_static_platform_ir()`.
- **Fix:** Host integration tests compile `configs/qemu.yaml`, plan layout, and pass `&layout` into Gate C entry points. UEFI hypervisor continues using snapshot reserve fields (VMX-only) from Phase 9.
- **Risk (deferred):** Mismatch between embedded snapshot reserve and full layout metadata cannot be detected on firmware until layout digest or compact layout blob is embedded.

### UEFI hypervisor entry

- **Finding:** Embedding full `StaticPlatformIR` in the hypervisor `.efi` would duplicate large compile-time metadata and expand the firmware image.
- **Disposition:** No change in Phase 10 — `hv-hypervisor-efi` continues `boot_from_transfer_and_init_vmx()`. Gate C remains host-tested with `configs/qemu.yaml` via `boot_from_transfer_and_init_gate_c()`.
- **Risk (deferred):** Production Gate C on OVMF requires embedded layout or a compact layout digest contract.

### Hypervisor re-exports (`hv-hypervisor`)

- **Finding:** Host integration tests should use a stable facade without reaching into `hv-hypervisor-boot` internals.
- **Fix:** `hv-hypervisor` re-exports `GateCInitResult`, `boot_check_and_init_gate_c`, and `boot_from_transfer_and_init_gate_c` alongside existing Gate B/VMX exports.
- **Risk (deferred):** Re-export surface will grow as hardware backends land; consider feature-gating hardware-only symbols later.

### Build / CI / OVMF smoke

- **Finding:** OVMF smoke validates Gate B closure under TCG; Gate C adds no new firmware requirements in this phase.
- **Fix:** Added workspace crates and host coverage tests; smoke config and serial evaluation unchanged. Hypervisor UEFI build pulls `hv-ept` / `hv-vtd` transitively via `hv-hypervisor-boot` but firmware entry does not invoke Gate C.
- **Risk (deferred):** KVM-backed OVMF with production requirements and Gate C success log line not exercised in CI.

## Findings and disposition

### MUST FIX (applied)

1. **EPT foundation crate** — `hv-ept` provides `EptInitPlan`, `plan_ept_init()`, `init_ept()` with mock/failing backends.
2. **VT-d foundation crate** — `hv-vtd` provides `VtdInitPlan`, `plan_vtd_init()`, `init_vtd()` with mock/failing backends.
3. **Gate C orchestration** — `hv-hypervisor-boot::gate_c` chains VMX + EPT + VT-d mock inits after Gate B validation.
4. **Host re-exports** — `hv-hypervisor` re-exports Gate C API for integration tests.

### SHOULD FIX (applied)

5. **Coverage and tests** — EPT/VT-d error paths, Gate C orchestration tests in `hv-hypervisor-boot`; workspace line coverage **95.31%** (≥ 95%).
6. **Documentation** — Architecture, platform contract, proof levels, and README updated for Phase 10 scope and deferred hardware work.

### Documented (deferred)

7. **Hardware VMXON/EPT paging** — Mock backends only; x86 VMX/EPT instructions remain future work.
8. **Hardware VT-d/IOMMU programming** — Mock backends only; DMAR-directed table setup remains future work.
9. **UEFI Gate C path** — Requires embedded `StaticPlatformIR` or compact layout binding in hypervisor image.
10. **Production OVMF Gate C smoke** — Host-tested with `configs/qemu.yaml`; KVM OVMF deferred.
11. **Gate C backend failure through orchestration** — `map_vmx_error` / `map_vtd_error` during `boot_*_gate_c()` not end-to-end tested; crate-level backend tests suffice for Phase 10.
12. **e1000 datapath (Gate D)** — Unchanged; no datapath work in Phase 10.

## Delivered

| Component | Role |
|-----------|------|
| `hv-ept` | EPT init plan, identity mappings, mock/failing backends, init orchestration |
| `hv-vtd` | VT-d init plan, PCI assignments, interrupt-remapping flag, mock/failing backends |
| `hv-hypervisor-boot::gate_c` | Gate B validation + VMX/EPT/VT-d mock init orchestration |
| `hv-hypervisor` | Re-exports `GateCInitResult`, `boot_*_gate_c` entry points |

## Verification

- `cargo test --workspace` — pass
- `cargo clippy --all-targets --all-features -- -D warnings` — pass
- `cargo xtask coverage` — pass (**95.31%** line coverage, threshold 95%)
- `cargo xtask build-boot-chain` — produces `build/boot-chain/hv-loader.efi` and `hv-hypervisor.efi`
- `cargo xtask ovmf-smoke-boot` — pass (Gate B unchanged; serial log: `hypervisor Gate B boot succeeded`)
- `cargo clippy --release --target x86_64-unknown-uefi --manifest-path crates/hv-loader-efi-bin/Cargo.toml -- -D warnings` — pass
- `cargo clippy --release --target x86_64-unknown-uefi --manifest-path crates/hv-hypervisor-efi-bin/Cargo.toml -- -D warnings` — pass

## Review status

All MUST and SHOULD items above are applied. Phase 9 deferrals #11 and #12 are partially closed (planning + mock backends; hardware programming deferred). Remaining deferred items are documented with explicit phase ownership. PR **#11** is ready for human review.
