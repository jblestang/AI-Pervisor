# Coverage policy

The workspace must maintain **>= 95% line coverage** measured by `cargo llvm-cov`.
This threshold is mandatory in CI and is not optional for merges.

## Commands

```bash
cargo xtask coverage
cargo xtask coverage --min-lines 95
```

Internally this runs:

```bash
cargo llvm-cov --workspace --summary-only --fail-under-lines 95
```

## Methodology

1. **Diagnose before fixing** — every uncovered line is mapped with `cargo llvm-cov --show-missing-lines` (or LCOV) and tied to a concrete code path before adding a test or removing dead code.
2. **No speculative fixes** — unreachable branches are removed only when their impossibility is proven (for example `u32` slot sizes cannot overflow `u64` metadata addition). Defensive error paths that are unreachable with current types are not “fixed” with artificial failures.
3. **Exhaustive fixtures** — YAML fixtures under `crates/hv-config-model/tests/fixtures/` cover syntax, semantic, normalization, arithmetic overflow, and datapath policy branches.
4. **Dispatch tested in libraries** — `hv-config` and `xtask` CLI parsing/mapping live in library code (`parse_*_command`, `map_cli_command`, `dispatch_*`) and are covered by unit tests; `main.rs` stays a thin entry point.
5. **No recursive subprocess tests** — tests must not spawn `xtask test` or `xtask coverage` without an environment guard, to avoid infinite CI loops.

## Scope

Coverage includes library code and host tools (`hv-config`, `xtask`).

Tests are organized as:

- unit tests in each module (`#[cfg(test)]`)
- integration tests under `crates/*/tests/` and `tools/*/tests/`
- YAML fixtures under `crates/hv-config-model/tests/fixtures/`

## Fixture catalogue (representative)

| Fixture | Exercised behaviour |
|---------|---------------------|
| `valid/smt_disabled.yaml` | `RawSmtPolicy::Disabled` normalization and requirements |
| `valid/datapath_device_without_role.yaml` | datapath policy with `role: None` devices |
| `valid/datapath_same_partition_gateway.yaml` | ingress/egress on same partition (`continue` path) |
| `invalid/guest_memory_sum_overflow.yaml` | guest RAM sum overflow in intent IR |
| `invalid/ipc_sum_overflow.yaml` | IPC shared-memory sum overflow in intent IR |
| `invalid/ipc_shared_overflow.yaml` | per-channel IPC multiply overflow |

## CI

The GitHub Actions workflow runs `cargo xtask coverage` after unit tests.
