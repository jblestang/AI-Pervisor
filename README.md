# AI-Pervisor

Static x86-64 hypervisor workspace (Phases 0–15: config pipeline, platform validation, boot path, UEFI Gate C, hardware programming, CPU seams, live instructions, REAL_HW resident install).

## Commands

```bash
cargo xtask test
cargo xtask coverage
cargo xtask fuzz
cargo xtask build-efi
cargo xtask build-boot-chain
cargo xtask build-boot-chain-live
cargo xtask ovmf-smoke-boot
cargo xtask live-qemu-smoke
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
- [Phase 10 expert review](docs/reviews/phase-10-expert-review.md)
- [Phase 11 expert review](docs/reviews/phase-11-expert-review.md)
- [Phase 12 expert review](docs/reviews/phase-12-expert-review.md)
- [Phase 13 expert review](docs/reviews/phase-13-expert-review.md)
- [Phase 14 expert review](docs/reviews/phase-14-expert-review.md)
- [Phase 15 expert review](docs/reviews/phase-15-expert-review.md)
- [Phase 16 expert review](docs/reviews/phase-16-expert-review.md)
- [OVMF boot](docs/ovmf-boot.md)
- [Fuzzing](docs/fuzzing.md)
- [No-panic policy](docs/no-panic.md)
