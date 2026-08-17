# Fuzzing

All byte- and text-oriented parsing entry points in the workspace are covered by libFuzzer targets under `fuzz/`.

## Running locally

```bash
cargo xtask fuzz
cargo xtask fuzz --runs 4096
```

This builds `fuzz/Cargo.toml` with `CXX=g++` (libFuzzer links C++ runtime code) and runs a smoke iteration over every target.

To run one target manually:

```bash
CXX=g++ cargo build --release --manifest-path fuzz/Cargo.toml
./fuzz/target/release/boot_info_parse -runs=4096
```

Optional seed inputs live under `fuzz/corpus/<target>/` and can be passed as trailing arguments to the binary.

## Targets and parsers

| Fuzz target | Crate | Parsing API |
|-------------|-------|-------------|
| `boot_info_parse` | `hv-boot-abi` | `BootInfoView::parse`, descriptor/section walks, `validate_rsdp_section` |
| `acpi_rsdp_parse` | `hv-boot-abi` | `AcpiRsdp::parse`, `validate_rsdp_section` |
| `uefi_descriptor_parse` | `hv-boot-abi` | `UefiMemoryDescriptor::parse` (single descriptor and strided map walks) |
| `acpi_walk` | `hv-acpi-walk` | `collect_acpi_tables` with `FirmwareMemoryImage` |
| `config_yaml` | `hv-config-model` | `load_raw_from_str`, `validate_syntax`, `validate_semantics` |
| `pci_bdf_parse` | `hv-config-model` | `parse_bdf` |
| `observed_platform_json` | `hv-platform-model` | `parse_observed_platform_json` |
| `observe_platform` | `hv-platform-model` | `observe_platform` (UEFI memory-map and ACPI capability scanning) |
| `transfer_parse` | `hv-boot-abi` | `HypervisorTransferView::parse`, `decode_observation_transfer` |

## Policy

- Fuzz harnesses must not panic; parsers return `Result` and harnesses ignore outcomes.
- Production crates keep the workspace no-panic denies; fuzz binaries live only under `fuzz/`.
- CI runs `cargo xtask fuzz` as a bounded smoke test (`512` runs per target by default).

## Proof level

Parsing surfaces listed above are validated at `UNIT + FUZZ` (see [proof-levels.md](proof-levels.md)).
