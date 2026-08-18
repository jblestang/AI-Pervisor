# Phase 10 expert review

Multi-domain review of Gate C foundation: EPT/VT-d init planning, mock backends, and host-tested Gate C orchestration (`cursor/phase-10-gate-c-foundation-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| EPT foundation | `hv-ept` init plan, identity mappings, mock backend, feature-gated init |
| VT-d foundation | `hv-vtd` init plan, PCI device assignments, mock backend, interrupt remapping flag |
| Gate C orchestration | `hv-hypervisor-boot::gate_c` chains VMX + EPT + VT-d mock inits after Gate B validation |
| Host boot path | `boot_from_transfer_and_init_gate_c()` requires `StaticPlatformIR` from compile-time planner |
| UEFI hypervisor entry | Unchanged from Phase 9 — still Gate B (`boot_from_transfer_and_init_vmx`) until layout is embedded |
| Build / CI | Workspace members `hv-ept`, `hv-vtd`; OVMF smoke unchanged (TCG-friendly config) |

## Phase 9 deferrals closed

| Phase 9 item | Phase 10 disposition |
|--------------|---------------------|
| Real VMXON/EPT/VT-d enablement (Phase 9 #11) | **Partially closed** — EPT/VT-d planning seams and mock backends added; hardware VMXON/EPT paging/IOMMU programming remain deferred. |
| VT-d / interrupt remapping enablement (Phase 9 #12) | **Partially closed** — `plan_vtd_init()` + `init_vtd()` validate capability and record assignments; no IOMMU register programming yet. |

## Domain expert notes

### EPT foundation (`hv-ept`)

- **Finding:** Gate C requires EPT hierarchy planning before hardware page-table programming; identity mappings must cover guest private and IPC regions from `StaticPlatformIR`.
- **Fix:** `plan_ept_init()` builds identity mappings with page-alignment checks; EPT root table is placed at `vmx_region + VMXON_REGION_MIN_BYTES` inside the hypervisor reserve. `MockEptBackend` records plan execution; `init_ept()` gates on validated EPT observation.
- **Risk (deferred):** No EPT paging structures, walk length, or memory-type attributes — mock backend only until hardware Gate C.

### VT-d foundation (`hv-vtd`)

- **Finding:** VT-d init must bind PCI BDFs to partition VM IDs and honor interrupt-remapping requirements from `PlatformRequirements`.
- **Fix:** `plan_vtd_init()` copies `layout.pci_devices` into `VtdDeviceAssignment`; `interrupt_remapping` follows `FeatureRequirement::Required | Preferred`. `MockVtdBackend` records assignments; `init_vtd()` gates on validated VT-d and interrupt-remapping observation.
- **Risk (deferred):** No DMAR parsing, root/context tables, or posted-interrupt programming.

### Gate C orchestration (`hv-hypervisor-boot`)

- **Finding:** Phase 9 orchestration covered VMX only; Gate C must chain EPT and VT-d init behind the same fail-closed validation without pulling hardware code into firmware yet.
- **Fix:** `boot_from_transfer_and_init_gate_c()` and `boot_check_and_init_gate_c()` run Gate B validation, plan VMX/EPT/VT-d, and invoke mock backends when features are required. `GateCInitResult` exposes all three plans plus validated platform state.
- **Risk (deferred):** Gate C entry requires `&StaticPlatformIR`, which is not embedded in the UEFI hypervisor image today; firmware path stays on Gate B until Phase 11+ layout embedding.

### UEFI hypervisor entry

- **Finding:** Embedding full `StaticPlatformIR` in the hypervisor `.efi` would duplicate large compile-time metadata and expand the firmware image.
- **Disposition:** No change in Phase 10 — `hv-hypervisor-efi` continues `boot_from_transfer_and_init_vmx()`. Gate C remains host-tested with `configs/qemu.yaml` via `boot_from_transfer_and_init_gate_c()`.
- **Risk (deferred):** Production Gate C on OVMF requires embedded layout or a compact layout digest contract.

### Build / CI / OVMF smoke

- **Finding:** OVMF smoke validates Gate B closure under TCG; Gate C adds no new firmware requirements in this phase.
- **Fix:** Added workspace crates and host coverage tests; smoke config and serial evaluation unchanged.
- **Risk (deferred):** KVM-backed OVMF with production requirements and Gate C success log line not exercised in CI.

## Findings and disposition

### MUST FIX (applied)

1. **EPT foundation crate** — `hv-ept` provides `EptInitPlan`, `plan_ept_init()`, `init_ept()` with mock/failing backends.
2. **VT-d foundation crate** — `hv-vtd` provides `VtdInitPlan`, `plan_vtd_init()`, `init_vtd()` with mock/failing backends.
3. **Gate C orchestration** — `hv-hypervisor-boot::gate_c` chains VMX + EPT + VT-d mock inits after Gate B validation.
4. **Host re-exports** — `hv-hypervisor` re-exports Gate C API for integration tests.

### SHOULD FIX (applied)

5. **Coverage and tests** — EPT/VT-d error paths, Gate C orchestration tests in `hv-hypervisor-boot`; workspace line coverage ≥ 95%.
6. **Documentation** — Architecture, platform contract, proof levels, and README updated for Phase 10 scope and deferred hardware work.

### Documented (deferred)

7. **Hardware VMXON/EPT paging** — Mock backends only; x86 VMX/EPT instructions remain future work.
8. **Hardware VT-d/IOMMU programming** — Mock backends only; DMAR-directed table setup remains future work.
9. **UEFI Gate C path** — Requires embedded `StaticPlatformIR` or compact layout binding in hypervisor image.
10. **Production OVMF Gate C smoke** — Host-tested with `configs/qemu.yaml`; KVM OVMF deferred.
11. **e1000 datapath (Gate D)** — Unchanged; no datapath work in Phase 10.

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
- `cargo xtask coverage` — pass (≥ 95% line coverage)
- `cargo xtask build-boot-chain` — produces `build/boot-chain/hv-loader.efi` and `hv-hypervisor.efi`
- `cargo xtask ovmf-smoke-boot` — pass (Gate B unchanged; serial log: `hypervisor Gate B boot succeeded`)

## Review status

All MUST and SHOULD items above are applied. Phase 9 deferrals #11 and #12 are partially closed (planning + mock backends; hardware programming deferred). Remaining deferred items are documented with explicit phase ownership.
