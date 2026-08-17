# AI-Pervisor

Static x86-64 hypervisor workspace (Phases 0–6: config pipeline, platform validation, boot path, ACPI walk).

## Commands

```bash
cargo xtask test
cargo xtask coverage
cargo xtask fuzz
cargo xtask build
cargo xtask config validate configs/qemu.yaml
cargo xtask config generate configs/qemu.yaml
cargo clippy --all-targets --all-features -- -D warnings
```

## Documentation

- [Architecture](docs/architecture.md)
- [Configuration](docs/configuration.md)
- [Platform contract](docs/platform-contract.md)
- [Boot ABI](docs/boot-abi.md)
- [Proof levels](docs/proof-levels.md)
- [Phase 5 expert review](docs/reviews/phase-5-expert-review.md)
- [Phase 6 expert review](docs/reviews/phase-6-expert-review.md)
- [Fuzzing](docs/fuzzing.md)
- [No-panic policy](docs/no-panic.md)
