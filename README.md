# AI-Pervisor

Static x86-64 hypervisor workspace (Phases 0–3 foundation).

## Commands

```bash
cargo xtask test
cargo xtask build
cargo xtask config validate configs/qemu.yaml
cargo xtask config generate configs/qemu.yaml
cargo clippy --all-targets --all-features -- -D warnings
```

## Documentation

- [Architecture](docs/architecture.md)
- [Configuration](docs/configuration.md)
- [Platform contract](docs/platform-contract.md)
- [Proof levels](docs/proof-levels.md)
