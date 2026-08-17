# Phases 0–3 expert review

Retrospective multi-domain review of Gate A foundation (`cursor/phases-0-3-foundation-0b4f`, tip `6b4c127`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Types / safe arithmetic | `hv-types` newtypes, `checked_*`, `align_up` |
| Configuration model | `hv-config-model` parse, syntax, semantic, normalize, requirements, StaticIntentIR, digest |
| Boot / guest ABI skeletons | `hv-boot-abi`, `hv-guest-abi` layout stability, compatibility checks |
| Host tooling | `hv-config` CLI, `xtask` wrappers, artifact generation |
| Safety policy | no-panic Clippy denies, error propagation, ≥95% coverage gate |
| Threat / proof scaffolding | `docs/threat-model.md`, `docs/proof-levels.md` |

## Findings and disposition

### Gate A delivery criteria (met at Phase 3)

1. **Configuration pipeline** — `compile_config()` runs syntax → semantic → normalize → `PlatformRequirements` → `StaticIntentIR` → `config_digest()` deterministically.
2. **Semantic fail-closed validation** — Duplicate partition IDs, PCI BDF ownership, IPC DAG acyclicity, guest image references, and core affinity conflicts are rejected or surfaced as explicit warnings.
3. **ABI layout contracts** — Boot and guest headers use `#[repr(C)]` with stable size/alignment tests and version/magic compatibility helpers only (no runtime parsing yet).
4. **No-panic production policy** — Workspace and crate-root Clippy denies on `unwrap`, `expect`, `panic`, unchecked indexing; CLIs map errors to exit codes.
5. **Coverage gate** — `cargo xtask coverage` enforces ≥95% line coverage with CI integration.

### SHOULD FIX (closed in later phases)

6. **Boot ABI blob parsing** — Phase 0–3 exported header/descriptor types only. Bounded `BootInfoView::parse()` and section walks landed in Phase 5 (`cursor/phase-5-boot-path-0b4f`); see [phase-5-expert-review.md](phase-5-expert-review.md).
7. **Configuration digest at boot** — Digest was computed and written to artifacts but not verified on a boot path until Phase 7 UEFI embedding (`HV_CONFIG_DIGEST_PATH` / `build/config.sha256`); see [phase-7-expert-review.md](phase-7-expert-review.md).
8. **Parser fuzzing** — `config_yaml` and `pci_bdf_parse` fuzz targets and CI smoke runs were added with Phase 5+ boot-path work; see [fuzzing.md](../fuzzing.md).
9. **Threat model depth** — `docs/threat-model.md` remains a stub defining trust boundaries; expansion to full safety/fault/timing models is deferred beyond Gate B.

### Documented (deferred at Gate A)

10. **Guest ABI parser** — `hv-guest-abi` still exposes layout types and compatibility checks only; guest-side boot info parsing remains future work (Gate C+).
11. **Runtime boot path** — No loader, hypervisor orchestration, or firmware observation at Gate A; intentionally deferred to Phase 5+.
12. **MIRI in CI** — `proof-levels.md` lists `MIRI` for newtypes/arithmetic, but no Miri job is wired in CI; property tests and Clippy provide partial coverage today.
13. **Observed platform / layout planning** — Platform validation and static physical layout are Gate B (Phase 4+); see [phase-4-expert-review.md](phase-4-expert-review.md).

## Delivered

| Component | Role |
|-----------|------|
| `hv-types` | Strong newtypes (`VmId`, `PciBdf`, `ByteSize`, …) and overflow-safe arithmetic |
| `hv-config-model` | YAML model, validation pipeline, normalization, requirements extraction, StaticIntentIR |
| `hv-boot-abi` | Versioned loader→hypervisor header and descriptor kinds (layout tests only) |
| `hv-guest-abi` | Versioned hypervisor→guest header and region descriptors (layout tests only) |
| `hv-config` | Host CLI: validate, compile, generate review artifacts |
| `xtask` | Developer wrappers (`test`, `coverage`, `config validate`, …) |
| `docs/no-panic.md` | Production no-panic policy and Clippy enforcement matrix |

## Verification

Reviewed at current workspace tip (includes later-phase fixes; Gate A crates unchanged in behavior):

- `cargo test -p hv-types -p hv-config-model -p hv-boot-abi -p hv-guest-abi` — pass
- `cargo clippy -p hv-types -p hv-config-model -p hv-boot-abi -p hv-guest-abi --all-targets -- -D warnings` — pass
- `cargo xtask config validate configs/qemu.yaml` — pass
- `cargo xtask coverage` — pass (≥ 95% workspace line coverage)

## Review status

Gate A delivery criteria are met. Items 6–9 were intentionally out of scope at Phase 3 and are closed or tracked in Phases 5–7 reviews. Remaining deferrals (10–13) stay documented with explicit later-phase ownership.
