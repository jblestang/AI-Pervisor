# No-panic policy

Production code must not panic intentionally.

## Rules

- No `unwrap()`, `expect()`, or `panic!()` in library or tool code outside tests.
- No unchecked indexing (`slice[i]`); use `.get(i)` or iterators and propagate errors.
- No `todo!()`, `unimplemented!()`, or `unreachable!()` in production paths.
- Errors are returned as `Result` or mapped to explicit process exit codes in CLIs.
- CLI argument parsing uses `try_parse()` and exits with code `2` on usage errors.

## Enforcement

Workspace Clippy lints (`Cargo.toml`):

- `unwrap_used = "deny"`
- `expect_used = "deny"`
- `panic = "deny"`
- `unreachable = "deny"`
- `todo = "deny"`
- `unimplemented = "deny"`
- `indexing_slicing = "deny"`

Critical crates repeat these denies at the crate root (`hv-types`, `hv-config-model`, ABIs, host tools).

Tests may use assertions and `expect` via `.clippy.toml` (`allow-*-in-tests`).

## Failure handling

| Context | Policy |
|---------|--------|
| Config validation | Return `ConfigError`, exit `1` from CLI |
| CLI usage error | Print clap error, exit `2` |
| Future hypervisor runtime | Fail-safe stop; no panic in steady state |
| Future guest runtime | Partition fault policy; no panic in datapath |

## Proof level

`REVIEW` + Clippy CI (`-D warnings`).
