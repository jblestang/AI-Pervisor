# Phase 12 expert review

Multi-domain review of Gate C hardware programming foundation: VMXON/EPT/VT-d structure encoding, programming backends, and host-tested Gate C programming orchestration (`cursor/phase-12-hw-programming-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| VMX programming | `program_vmxon_region()`, `ProgrammingVmxBackend`, `VmxonProgrammedRegion` |
| EPT programming | `program_ept_tables()`, `ProgrammingEptBackend`, identity entry encoding |
| VT-d programming | `program_vtd_tables()`, `ProgrammingVtdBackend`, context entry encoding |
| Gate C programming orchestration | `boot_*_gate_c_programming*()` with `GateCProgrammingResult` |
| UEFI hypervisor entry | Unchanged — still mock Gate C from Phase 11 (no structure programming in firmware yet) |
| Build / CI | Host-tested programming path; OVMF smoke unchanged |

## Phase 11 deferrals closed

| Phase 11 item | Phase 12 disposition |
|---------------|---------------------|
| Hardware VMXON/EPT paging / VT-d IOMMU (Phase 11 #7) | **Partially closed** — structure programming backends encode VMXON/EPT/VT-d metadata; no VMX/EPT/VT-d CPU instructions yet. |
| Layout snapshot digest seal (Phase 11 #8) | **Unchanged** — still co-generated; independent digest remains deferred. |

## Code coverage

Fresh run: `cargo xtask coverage` (2026-08-18).

| Metric | Value |
|--------|-------|
| Workspace line coverage | **95.00%** (7796 lines, 390 missed) |
| Minimum threshold | 95% |
| Result | **pass** |

## Domain expert notes

### VMX programming (`hv-vmx`)

- **Finding:** Gate C hardware bring-up requires encoding VMXON region contents before any VMXON instruction executes.
- **Fix:** `program_vmxon_region()` writes revision prefix into a page-sized buffer; `ProgrammingVmxBackend` implements `VmxBackend` by recording programmed bytes.
- **Risk (deferred):** No `vmxon`/`vmclear` instructions; revision ID is a MODEL reference constant, not read from `IA32_VMX_BASIC`.

### EPT programming (`hv-ept`)

- **Finding:** EPT init must produce root table bytes and per-mapping encoded entries before EPT pointer activation.
- **Fix:** `program_ept_tables()` builds root page bytes and `EptProgrammedMapping` records with R/W/X + WB memory type encoding; `ProgrammingEptBackend` stores `EptProgrammedTables`.
- **Risk (deferred):** Simplified single-entry root encoding; full multi-level EPT walks and memory-type policies remain future work.

### VT-d programming (`hv-vtd`)

- **Finding:** VT-d init must record PCI→VM assignments and interrupt-remapping intent before IOMMU register programming.
- **Fix:** `program_vtd_tables()` emits `VtdProgrammedAssignment` records with MODEL context flags; `ProgrammingVtdBackend` stores programmed output.
- **Risk (deferred):** No DMAR register access, root/context table memory writes, or posted-interrupt setup.

### Gate C programming orchestration

- **Finding:** Host path needs Gate C orchestration with programming backends distinct from mock MODEL backends used on UEFI.
- **Fix:** `GateCProgrammingResult` wraps `GateCInitResult` plus optional programmed VMX/EPT/VT-d artifacts; `boot_from_transfer_and_init_gate_c_programming*()` host entry points added.
- **Risk (deferred):** Programming and mock paths duplicate orchestration; future refactor may unify via backend trait objects.

### UEFI / firmware path

- **Finding:** Firmware images remain `no_std` with workspace `unsafe_code = deny`; structure programming uses `alloc` vectors on host.
- **Disposition:** UEFI hypervisor continues Phase 11 mock Gate C; programming backends are host-tested only in Phase 12.
- **Risk (deferred):** Firmware hardware programming requires fixed buffers and likely a dedicated unsafe host/firmware split crate.

## Findings and disposition

### MUST FIX (applied)

1. **VMX programming backend** — `ProgrammingVmxBackend` + `program_vmxon_region()`.
2. **EPT programming backend** — `ProgrammingEptBackend` + `program_ept_tables()`.
3. **VT-d programming backend** — `ProgrammingVtdBackend` + `program_vtd_tables()`.
4. **Gate C programming orchestration** — `GateCProgrammingResult` and host `boot_*_programming*` entry points.

### SHOULD FIX (applied)

5. **Coverage and tests** — Programming unit/integration tests; workspace line coverage **95.00%**.
6. **Documentation** — Architecture, platform contract, proof levels, README updated.

### Documented (deferred)

7. **VMX/EPT/VT-d CPU instructions** — No `vmxon`, EPT pointer load, or IOMMU enable MMIO.
8. **UEFI hardware programming** — Mock Gate C on firmware; programming backends host-only.
9. **Production KVM OVMF hardware path** — Not exercised in CI.
10. **e1000 datapath (Gate D)** — Unchanged.

## Delivered

| Component | Role |
|-----------|------|
| `hv-vmx::program` | VMXON region encoding + `ProgrammingVmxBackend` |
| `hv-ept::program` | EPT root/mapping encoding + `ProgrammingEptBackend` |
| `hv-vtd::program` | VT-d assignment encoding + `ProgrammingVtdBackend` |
| `hv-hypervisor-boot::gate_c` | `GateCProgrammingResult`, `boot_*_gate_c_programming*` |

## Verification

- `cargo test --workspace` — pass
- `cargo clippy --all-targets --all-features -- -D warnings` — pass
- `cargo xtask coverage` — pass (**95.00%** line coverage)
- `cargo xtask build-boot-chain` — pass
- `cargo xtask ovmf-smoke-boot` — pass

## Review status

All MUST and SHOULD items above are applied. Phase 11 hardware deferral is partially closed (structure programming; CPU instructions deferred). PR **#13** is ready for human review.
