# Benchmark specification

Performance validation uses the official metric defined here. Phase 21 adds host mock and wall-clock benchmark harnesses. Phase 22 adds guest-driven datapath runtime under VMX (validate-only default). Phase 23 adds real `guests/` source-tree ELFs and `cargo xtask datapath-live-benchmark` (build guests + wall-clock). Phase 24 adds guest boot-info install and VMCS RDI handoff (`datapath-guest-live`). Phase 25 adds live VMX guest execution scaffolding (`datapath-guest-execution`). Phase 26 adds in-VM guest throughput measurement via the guest runtime relay path (`datapath-guest-throughput`); host CI uses mock timing, REAL_HW firmware may execute live measurement. Phase 27 adds sustained 64-frame guest relay loops in freestanding firmware and live throughput wiring via `datapath-guest-relay-live`.

## Official throughput metric

```text
throughput = useful UDP payload bytes received at OUT egress
```

Not Ethernet L2 bytes. Not IP header bytes unless explicitly changed in a future revision.

## Protocol

- UDP over IPv4
- Fixed frame and payload sizes recorded per run
- Measurement taken at OUT, not at ingress

## Procedure

- warmup: 10 s
- measurement: 30 s
- runs: at least 5
- publish minimum, mean, median, and p95 when relevant
- success must not depend on a single exceptional run

## Environment metadata to record

- QEMU version
- OVMF version
- accelerator (`tcg`, `kvm`, ...)
- host CPU count and affinities
- configuration digest (`config.sha256`)

## Target

Reproducible throughput of at least **200 Mbit/s** on the path:

```text
e1000-IN -> IN -> MID -> OUT -> e1000-OUT
```

MID must process every traversing unit.
