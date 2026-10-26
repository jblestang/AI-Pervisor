# ICD — Interface Control Document

**Product:** Hyperviseur edge gateway (spike)  
**Version:** 0.1 (Task 0 seed)  
**Language:** English

## Table of contents

1. [Purpose](#1-purpose)
2. [Conventions](#2-conventions)
3. [Shared ring ABI (`IF-RING-*`)](#3-shared-ring-abi-if-ring-)
4. [NicBackend (`IF-NIC-*`)](#4-nicbackend-if-nic-)
5. [Boot info blob (`IF-BOOT-*`)](#5-boot-info-blob-if-boot-)
6. [UART (`IF-UART-*`)](#6-uart-if-uart-)
7. [Stats page (`IF-STATS-*`)](#7-stats-page-if-stats-)
8. [Config schema `demo.toml` (`IF-CFG-*`)](#8-config-schema-demotoml-if-cfg-)
9. [Serial smoke protocol (`IF-SER-*`)](#9-serial-smoke-protocol-if-ser-)
10. [Interface index](#10-interface-index)

## 1. Purpose

Define **wire and logical interfaces** owned by documentation, not by hardcoded source literals. Implementations must match these IDs; SRS `SWR-*` and STD `TEST-*` cite them.

Endianness: little-endian. Address width: 64-bit guest physical unless noted.

## 2. Conventions

| Prefix | Domain |
|--------|--------|
| `IF-RING-` | Shared ring layout and operations |
| `IF-NIC-` | `NicBackend` / VF / Inject |
| `IF-BOOT-` | Boot info blob from `hv` to guests |
| `IF-UART-` | Emulated UART / hypercall logging |
| `IF-STATS-` | Shared counters page |
| `IF-CFG-` | `demo.toml` + env |
| `IF-SER-` | Host-visible serial protocol for smoke |

Exact Rust field names may use snake_case equivalents; **layout and semantics** here are normative for the spike.

## 3. Shared ring ABI (`IF-RING-*`)

### IF-RING-001 — Frame descriptor

| Field | Type | Meaning |
|-------|------|---------|
| `offset` | `u32` | Byte offset into the shared payload arena |
| `len` | `u32` | Frame length in bytes |

Descriptors do not embed payload bytes (zero-copy intent).

### IF-RING-002 — Ring header (shared memory)

Logical header (exact packing finalized in `ring` crate; must remain SPSC-safe):

| Field | Type | Meaning |
|-------|------|---------|
| `capacity` | power-of-two `usize`/`u32` | Slot count (`capacity_pow2` from config) |
| `write_idx` | atomic index | Producer cursor |
| `read_idx` | atomic index | Consumer cursor |

Capacity and arena size come from config (`IF-CFG-RING`), not source constants.

### IF-RING-003 — Operations

| Op | Semantics |
|----|-----------|
| `try_push(desc)` | Enqueue; on full → error / drop path (see SWR-013) |
| `try_pop()` | Dequeue; on empty → error |

Single producer / single consumer per direction. Bidirectional datapath uses two rings or a dual-ring layout with the same descriptor ABI (implementation choice documented in SSDD as-built).

### IF-RING-004 — Payload arena

Contiguous shared region; `frame_capacity` (config) is the maximum `len` accepted for spike benches. Arena base/size appear in boot info (`IF-BOOT-002`).

## 4. NicBackend (`IF-NIC-*`)

### IF-NIC-001 — Trait (logical)

```text
rx_ready() -> bool
try_recv(buf) -> Result<usize, NicError>   # bytes written
try_send(frame) -> Result<(), NicError>
```

### IF-NIC-002 — Backends

| Name | Behavior |
|------|----------|
| `Sriov` | Real VF via VFIO / passthrough as configured |
| `Inject` | In-process frame queue (degraded lab / no VF) |

### IF-NIC-003 — Open policy

| Config | Device present | Result |
|--------|----------------|--------|
| `backend=sriov` | yes | `Sriov` |
| `backend=sriov`, `allow_inject_fallback=false` | no | **Error** |
| `backend=sriov`, `allow_inject_fallback=true` | no | `Inject` |
| `backend=inject` | N/A | `Inject` |

Flattened NIC config for guests is summarized in boot info (`IF-BOOT-003`); host-side open uses full `DemoConfig`.

### IF-NIC-004 — Errors

`NicError` includes at least: device missing, buffer too small, queue full/empty, I/O failure. Exact enum is code-owned; fail-fast on missing Sriov is mandatory (SWR-023).

## 5. Boot info blob (`IF-BOOT-*`)

Prepared by `hv` from `DemoConfig` before guest entry. Guests must not hardcode these values.

### IF-BOOT-001 — Identity

| Field | Meaning |
|-------|---------|
| `magic` | Spike-defined constant identifying the blob |
| `version` | Schema version (`u32`) |
| `guest_role` | `WAN` or `LAN` |

### IF-BOOT-002 — Memory map pointers

| Field | Meaning |
|-------|---------|
| `ring_header_gpa` | Guest physical address of ring header(s) |
| `arena_gpa` / `arena_len` | Payload arena |
| `stats_gpa` | Stats page (`IF-STATS-*`) |
| `boot_info_gpa` | Self / mirror as needed |

Private heap/stack regions remain guest-local; only shared regions are listed here.

### IF-BOOT-003 — NIC flattened config

| Field | Meaning |
|-------|---------|
| `backend` | `sriov` \| `inject` |
| `vf_bdf` | Optional BDF string/bytes when Sriov |
| Backend-specific MMIO / queue tips | Filled by `hv` from lab config |

### IF-BOOT-004 — UART

| Field | Meaning |
|-------|---------|
| `uart_port` or hypercall id | How guests emit logs (`IF-UART-*`) |

## 6. UART (`IF-UART-*`)

### IF-UART-001

Guests write log bytes via the mechanism advertised in boot info (I/O port exit and/or hypercall). `hv` emulates UART and forwards to the QEMU serial device visible to `xtask smoke` / operators.

### IF-UART-002

No requirement for interactive console input in the spike. N/A for spike: full 16550 feature parity.

## 7. Stats page (`IF-STATS-*`)

Shared page (or small region) mapped to both guests and readable by the harness/bench path.

### IF-STATS-001 — Counters (minimum)

| Counter | Direction / meaning |
|---------|---------------------|
| `wan_rx` / `wan_tx` | Frames at WAN NicBackend |
| `lan_rx` / `lan_tx` | Frames at LAN NicBackend |
| `ring_fwd_wan_to_lan` | Descriptors pushed WAN→LAN |
| `ring_fwd_lan_to_wan` | Descriptors pushed LAN→WAN |
| `ring_drop_full` | Drops due to full ring |
| `bytes_*` | Optional byte counters for bench |

Exact layout (`#[repr(C)]`) is finalized with code; this list is the semantic contract for smoke/bench.

### IF-STATS-002 — Synchronization

Counters are monotonic `u64` updated by the owning guest; readers tolerate torn reads for smoke; bench sampling interval is harness-defined.

## 8. Config schema `demo.toml` (`IF-CFG-*`)

Normative sections — all mandatory unless noted. Matches plan Task 1.

### IF-CFG-LAB

```toml
[lab]
qemu_bin = "qemu-system-x86_64"
memory_mb = 4096
smp = 4
enable_iommu = true
```

### IF-CFG-DATAPATH

```toml
[datapath]
backend = "sriov"              # "sriov" | "inject"
allow_inject_fallback = false
wan_vf_bdf = "0000:01:10.0"    # required if backend=sriov
lan_vf_bdf = "0000:01:10.1"
```

### IF-CFG-GUESTS

```toml
[guests]
wan_memory_mb = 128
lan_memory_mb = 128
wan_vcpus = 1
lan_vcpus = 1
```

### IF-CFG-RING

```toml
[ring]
capacity_pow2 = 1024
frame_capacity = 2048
```

### IF-CFG-BENCH

```toml
[bench]
frame_size = 64
duration_secs = 30
bidirectional = true
target_gbps = 1.0
```

### IF-CFG-PATHS

```toml
[paths]
hv_image = "target/x86_64-unknown-none/release/hv"
guest_wan_image = "target/x86_64-unknown-none/release/guest-wan"
guest_lan_image = "target/x86_64-unknown-none/release/guest-lan"
```

### IF-CFG-ENV — Environment overrides

| Variable | Effect |
|----------|--------|
| `HV_DEMO_CONFIG` | Path to TOML file |
| `HV_VF_BACKEND` | Override `datapath.backend` |
| `HV_WAN_VF_BDF` | Override `datapath.wan_vf_bdf` |
| `HV_LAN_VF_BDF` | Override `datapath.lan_vf_bdf` |

### IF-CFG-REJECT

Parser shall error if any mandatory field is missing (SWR-031). Example complete file: `config/demo.example.toml` (Task 1).

## 9. Serial smoke protocol (`IF-SER-*`)

### IF-SER-001

After a successful smoke datapath, the system emits the ASCII magic string `SMOKE_OK` on the UART/serial stream observed by `cargo xtask smoke`.

### IF-SER-002

N/A for spike: multi-line structured serial JSON protocol (bench uses file report instead).

## 10. Interface index

| ID | Summary |
|----|---------|
| IF-RING-001 | FrameDesc |
| IF-RING-002 | Ring header |
| IF-RING-003 | try_push / try_pop |
| IF-RING-004 | Payload arena |
| IF-NIC-001 | NicBackend ops |
| IF-NIC-002 | Sriov / Inject |
| IF-NIC-003 | Open policy |
| IF-NIC-004 | NicError |
| IF-BOOT-001 | Magic / version / role |
| IF-BOOT-002 | GPA map |
| IF-BOOT-003 | Flattened NIC |
| IF-BOOT-004 | UART tip |
| IF-UART-001 | Guest→hv log path |
| IF-UART-002 | No interactive RX (N/A) |
| IF-STATS-001 | Counter set |
| IF-STATS-002 | Sync rules |
| IF-CFG-LAB | `[lab]` |
| IF-CFG-DATAPATH | `[datapath]` |
| IF-CFG-GUESTS | `[guests]` |
| IF-CFG-RING | `[ring]` |
| IF-CFG-BENCH | `[bench]` |
| IF-CFG-PATHS | `[paths]` |
| IF-CFG-ENV | Env overrides |
| IF-CFG-REJECT | Incomplete reject |
| IF-SER-001 | `SMOKE_OK` |
| IF-SER-002 | Structured serial N/A |

**Open interface decisions:** Exact `#[repr(C)]` byte layouts — deferred to implementation commits; semantics above are binding.
