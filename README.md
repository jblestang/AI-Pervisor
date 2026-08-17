# AI-Pervisor

Static x86-64 hypervisor workspace (Phases 0–7: config pipeline, platform validation, boot path, ACPI walk, UEFI loader).

## Commands

```bash
cargo xtask test
cargo xtask coverage
cargo xtask fuzz
cargo xtask build-efi
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
- [Phase 7 expert review](docs/reviews/phase-7-expert-review.md)
- [OVMF boot](docs/ovmf-boot.md)
- [Fuzzing](docs/fuzzing.md)
- [No-panic policy](docs/no-panic.md)
