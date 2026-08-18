# Phase 21 expert review

Multi-domain review of Gate D datapath benchmark: mock throughput measurement, host wall-clock benchmark harness, and `datapath-benchmark` feature chain (`cursor/phase-21-datapath-benchmark-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Benchmark metric + procedure | `docs/benchmark.md`, `TARGET_THROUGHPUT_MBIT_PER_SEC`, warmup/measurement/runs |
| Mock benchmark | `run_mock_datapath_benchmark`, deterministic no_std timing |
| Host benchmark | `cargo xtask datapath-benchmark`, wall-clock measurement |
| Gate D benchmark orchestration | `GateDDatapathBenchmarkResult`, `boot_*_gate_d_datapath_benchmark*()` |
| UEFI + xtask | `datapath-benchmark` feature chain, serial markers, coverage pass |

## Phase 20 deferrals closed

| Phase 20 item | Phase 21 disposition |
|---------------|---------------------|
| 200 Mbit/s performance benchmark | **Partially closed** — mock validate-only + host wall-clock harness; firmware uses deterministic mock timing |
| Live guest datapath runtime | **Unchanged** — host mock path only |
| Real `guests/` source trees | **Unchanged** |

## Feature matrix

| Feature | Crate | Default | UEFI chain | Effect |
|---------|-------|---------|------------|--------|
| `datapath-benchmark` | `hv-hypervisor-boot` | off | off | Gate D benchmark orchestration atop `datapath-guests` |
| `datapath-benchmark` | `hv-hypervisor-efi` | off | opt-in | Benchmark boot entry + target-met marker |
| `datapath-benchmark` | `hv-hypervisor-efi-bin` | off | opt-in | Builds on `datapath-guests` |

## Serial markers

- `GATE_D_DATAPATH_BENCHMARK_MARKER` — benchmark orchestration succeeded
- `GATE_D_BENCHMARK_TARGET_MET_MARKER` — minimum run met 200 Mbit/s target

## Verification

- `cargo test --workspace` — pass
- `cargo test -p hv-datapath` — pass (benchmark fixtures)
- `cargo test -p hv-hypervisor-boot --features datapath-benchmark` — pass
- `cargo test -p hv-hypervisor-efi --features datapath-benchmark` — pass
- `cargo clippy --all-targets --all-features -- -D warnings` — pass
- `cargo xtask coverage` — multi-pass includes `datapath-benchmark`
- `cargo xtask build-boot-chain-live` — builds with full `datapath-*` chain including `datapath-benchmark`
- `cargo xtask datapath-benchmark` — host wall-clock benchmark (manual / CI optional)

## Review status

Phase 21 closes the performance benchmark scaffolding gap from Phase 20: official metric enforcement, mock no_std benchmark for firmware validate-only paths, host wall-clock harness via xtask, and Gate D benchmark orchestration with serial markers. Live guest datapath runtime under VMX remains deferred to Phase 22+.
