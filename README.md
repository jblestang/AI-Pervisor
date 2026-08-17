# AI-Pervisor

Static x86-64 hypervisor workspace (Phases 0–5: config pipeline, platform validation, boot path).

## Commands

```bash
cargo xtask test
cargo xtask coverage
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
- [No-panic policy](docs/no-panic.md)
