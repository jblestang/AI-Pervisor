# Coverage policy

The workspace must maintain **>= 95% line coverage** measured by `cargo llvm-cov`.

## Commands

```bash
cargo xtask coverage
cargo xtask coverage --min-lines 95
```

Internally this runs:

```bash
cargo llvm-cov --workspace --summary-only --fail-under-lines 95
```

## Scope

Coverage includes library code and host tools (`hv-config`, `xtask` libraries and thin CLI wrappers).

Tests are organized as:

- unit tests in each module (`#[cfg(test)]`)
- integration tests under `crates/*/tests/` and `tools/*/tests/`
- YAML fixtures under `crates/hv-config-model/tests/fixtures/`

## CI

The GitHub Actions workflow runs `cargo xtask coverage` after unit tests.
