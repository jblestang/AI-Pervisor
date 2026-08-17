# AI-Pervisor

Static x86-64 hypervisor workspace (Phases 0–9: config pipeline, platform validation, boot path, ACPI walk, UEFI loader, VMX foundation).

## Commands

```bash
cargo xtask test
cargo xtask coverage
cargo xtask fuzz
cargo xtask build-efi
cargo xtask build-boot-chain
cargo xtask ovmf-smoke-boot
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
- [Phase 8 expert review](docs/reviews/phase-8-expert-review.md)
- [Phase 9 expert review](docs/reviews/phase-9-expert-review.md)
- [OVMF boot](docs/ovmf-boot.md)
- [Fuzzing](docs/fuzzing.md)
- [No-panic policy](docs/no-panic.md)
