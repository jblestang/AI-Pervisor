# Benchmark specification (stub)

Performance validation begins after the full datapath exists. This document records the official metric now so later phases do not mix definitions.

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
