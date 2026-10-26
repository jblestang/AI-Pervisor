# Interface Control Document (ICD)

| Field | Value |
|-------|-------|
| **Document ID** | HV-EDGE-ICD-001 |
| **Title** | Hypervisor Edge Gateway — Interface Control Document |
| **Issue** | 0.4 |
| **Date** | 2026-10-26 |
| **Classification** | UNCLASSIFIED — Technology Demonstrator |
| **Parent suite** | HV-EDGE-DOC-SUITE |
| **Normative for** | Shared-memory layouts, NIC open policy, boot blob semantics, config schema, serial smoke token, bench report fields |
| **Transfer note** | Shared ring/arena/stats are the **only** intentional cross-domain coupling (safety partitioning surface) |

---

## Table of contents

1. [Document control](#1-document-control)  
2. [General interface conventions](#2-general-interface-conventions)  
3. [Shared ring (`IF-RING-*`)](#3-shared-ring-if-ring-)  
4. [NicBackend (`IF-NIC-*`)](#4-nicbackend-if-nic-)  
5. [Boot information blob (`IF-BOOT-*`)](#5-boot-information-blob-if-boot-)  
6. [UART (`IF-UART-*`)](#6-uart-if-uart-)  
7. [Statistics page (`IF-STATS-*`)](#7-statistics-page-if-stats-)  
8. [Configuration schema (`IF-CFG-*`)](#8-configuration-schema-if-cfg-)  
9. [Serial smoke protocol (`IF-SER-*`)](#9-serial-smoke-protocol-if-ser-)  
10. [Sequence charts](#10-sequence-charts)  
11. [Interface index](#11-interface-index)  
12. [Change record](#12-change-record)

---

## 1. Document control

### 1.1 Precedence

1. This ICD is **normative** for interface semantics and, where sized, binary layouts.  
2. In case of conflict with source comments, **this ICD prevails** until source and ICD are updated in the same baseline.  
3. SSS/SRS cite ICD IDs; they do not redefine layouts.

### 1.2 Maturity of layouts

| Interface | Binary layout status |
|-----------|----------------------|
| `IF-RING-*` | **Normative packing** aligned with `ring` crate Issue 0.2 |
| `IF-STATS-*` | Semantic counters + normative `#[repr(C)]` packing (`IF-STATS-003`) |
| `IF-BOOT-*` | Field set normative; packing at Tasks 5–6 |
| `IF-NIC-*` | Logical API + open policy normative |
| `IF-CFG-*` | TOML schema normative (see `config/demo.example.toml`) |

---

## 2. General interface conventions

| Topic | Convention |
|-------|------------|
| Endianness | Little-endian |
| Address width | 64-bit guest physical (GPA) unless stated |
| Atomic indices | `AtomicU32` free-running counters; wrap on overflow |
| Alignment | Structures aligned to their natural Rust/`repr(C)` alignment unless stated |
| String BDFs | ASCII `BBBB:bb:dd.f` (e.g. `0000:01:10.0`) |

---

## 3. Shared ring (`IF-RING-*`)

### IF-RING-001 — Frame descriptor

**Normative packing** (`#[repr(C)]`), 8 bytes:

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | `offset` | Byte offset into shared payload arena |
| 4 | 4 | `len` | Frame length in bytes (≤ `frame_capacity` from config) |

Payload bytes are **not** embedded in the descriptor (zero-copy).

### IF-RING-002 — Ring memory layout

```text
[ RingHeader | FrameDesc × capacity ]
```

**RingHeader** (`#[repr(C)]`), 16 bytes:

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | `capacity` | Slot count; non-zero power of two |
| 4 | 4 | `_pad` | Reserved; write as 0 |
| 8 | 4 | `write_idx` | Producer cursor (`AtomicU32`) |
| 12 | 4 | `read_idx` | Consumer cursor (`AtomicU32`) |

Slot index = `idx & (capacity - 1)`.

**Full:** `write_idx.wrapping_sub(read_idx) == capacity`  
**Empty:** `write_idx == read_idx`

Initial state after `new_in`: both indices 0; slots zeroed.

### IF-RING-003 — Operations

| Operation | Precondition | Success | Failure |
|-----------|--------------|---------|---------|
| `try_push(desc)` | Single producer | Enqueues descriptor; advances `write_idx` with release semantics | `RingFull` — caller drops / retries; no overwrite |
| `try_pop()` | Single consumer | Returns descriptor; advances `read_idx` with acquire semantics | `RingEmpty` |

**Concurrency:** Exactly one producer and one consumer per ring instance. Cross-guest mappings **shall** provide coherent shared pages (hypervisor responsibility).

### IF-RING-004 — Payload arena

Contiguous shared region separate from the descriptor ring. `FrameDesc.offset` is relative to arena base. Maximum accepted `len` for the demonstrator is `ring.frame_capacity` from configuration.

### IF-RING-005 — Initialisation errors

`RingInitError` when: capacity is 0 or not power-of-two; arena too small for header + slots; alignment cannot be satisfied.

### IF-RING-006 — Bidirectional datapath

The system **shall** provide logical forwarding in both WAN→LAN and LAN→WAN directions. Realisation **may** use two SPSC rings or an equivalent dual structure; each direction **shall** obey `IF-RING-001..005`.

---

## 4. NicBackend (`IF-NIC-*`)

### IF-NIC-001 — Logical interface

| Method | Semantics |
|--------|-----------|
| `rx_ready() -> bool` | True if a receive attempt may succeed without waiting |
| `try_recv(buf) -> Result<usize, NicError>` | Copies/produces at most `buf.len()` bytes; returns length |
| `try_send(frame) -> Result<(), NicError>` | Submits frame bytes for host-facing transmit |

### IF-NIC-002 — Backend types

| Name | Description |
|------|-------------|
| `Sriov` | Host↔guest via SR‑IOV Virtual Function (VFIO/passthrough as lab-configured) |
| `Inject` | In-process frame queue implementing the same interface (degraded / no-VF lab) |

Inject is a **first-class backend**, not a unit-test mock of Sriov.

### IF-NIC-003 — Open policy (normative)

| `datapath.backend` | VF present | `allow_inject_fallback` | Result |
|--------------------|------------|-------------------------|--------|
| `sriov` | yes | any | `Sriov` |
| `sriov` | no | `false` | **Error** (fail fast) |
| `sriov` | no | `true` | `Inject` |
| `inject` | any | any | `Inject` |

### IF-NIC-004 — Error classes

Implementations **shall** distinguish at least: device missing / open failure; buffer too small; queue full; queue empty; I/O failure. Exact Rust enum is implementation-owned; policy in `IF-NIC-003` is not optional.

---

## 5. Boot information blob (`IF-BOOT-*`)

Prepared by `hv` from `DemoConfig` before guest entry. Guests **shall not** hardcode these values.

### IF-BOOT-001 — Identity

| Field | Type (logical) | Description |
|-------|----------------|-------------|
| `magic` | `u32` | Identifies spike boot blob |
| `version` | `u32` | Schema version; increment on breaking layout |
| `guest_role` | enum/`u32` | `WAN` or `LAN` |

### IF-BOOT-002 — Memory map

| Field | Description |
|-------|-------------|
| `ring_wan_to_lan_gpa` / `ring_lan_to_wan_gpa` (or equivalent) | GPA of ring region(s) |
| `arena_gpa`, `arena_len` | Payload arena |
| `stats_gpa` | Statistics page |
| Private region bases/sizes | Informational / debug as needed |

### IF-BOOT-003 — Flattened NIC parameters

| Field | Description |
|-------|-------------|
| `backend` | `sriov` \| `inject` |
| `vf_bdf` | Present when Sriov |
| MMIO / queue tips | Filled by hypervisor from lab binding |

### IF-BOOT-004 — UART tip

| Field | Description |
|-------|-------------|
| `uart_port` and/or hypercall identifier | Guest log emission path (`IF-UART-001`) |

**Packing note:** Exact `#[repr(C)]` offsets are baselined when guest bring-up lands; field set above is binding.

---

## 6. UART (`IF-UART-*`)

### IF-UART-001 — Guest to operator

Guests write log bytes via the mechanism advertised in boot info. Hypervisor emulates UART (I/O exit and/or hypercall) and forwards to the QEMU serial device observed by `xtask smoke` and operators.

### IF-UART-002 — Input

Interactive console input is **N/A for demonstrator**. Full 16550 feature parity is **not** required.

---

## 7. Statistics page (`IF-STATS-*`)

### IF-STATS-001 — Counter set (semantic)

Monotonic `u64` counters:

| Counter | Meaning |
|---------|---------|
| `wan_rx`, `wan_tx` | Frames at WAN NicBackend |
| `lan_rx`, `lan_tx` | Frames at LAN NicBackend |
| `ring_fwd_wan_to_lan` | Descriptors successfully pushed WAN→LAN |
| `ring_fwd_lan_to_wan` | Descriptors successfully pushed LAN→WAN |
| `ring_drop_full` | Frames dropped due to full ring |
| `bytes_wan_rx`, `bytes_wan_tx`, `bytes_lan_rx`, `bytes_lan_tx` | Byte counters for bench |

### IF-STATS-002 — Observation rules

Readers (harness) **may** observe torn 64-bit values during smoke. Bench sampling: take a full counter snapshot at measurement start and end; compute saturating deltas (`IF-STATS` via `StatsSnapshot`). Counters are updated by the owning guest (or L1 inject / host Inject harness exercising the same forward code) only.

### IF-STATS-003 — Packed layout (`#[repr(C)]`)

Little-endian `u64` fields, total size **88** bytes (`STATS_PAGE_SIZE`). Offsets match `guest_common::stats_off`:

| Offset | Field |
|--------|-------|
| 0 | `wan_rx` |
| 8 | `wan_tx` |
| 16 | `lan_rx` |
| 24 | `lan_tx` |
| 32 | `ring_fwd_wan_to_lan` |
| 40 | `ring_fwd_lan_to_wan` |
| 48 | `ring_drop_full` |
| 56 | `bytes_wan_rx` |
| 64 | `bytes_wan_tx` |
| 72 | `bytes_lan_rx` |
| 80 | `bytes_lan_tx` |

---

## 8. Configuration schema (`IF-CFG-*`)

All sections mandatory unless noted. Example: `config/demo.example.toml`.

### IF-CFG-LAB

| Key | Type | Description |
|-----|------|-------------|
| `qemu_bin` | string | QEMU executable |
| `memory_mb` | integer | Nested VM RAM |
| `smp` | integer | vCPU count for QEMU |
| `enable_iommu` | bool | Pass IOMMU device when true |

### IF-CFG-DATAPATH

| Key | Type | Description |
|-----|------|-------------|
| `backend` | `"sriov"` \| `"inject"` | Backend selection |
| `allow_inject_fallback` | bool | See `IF-NIC-003` |
| `wan_vf_bdf` | string | Required when `backend=sriov` |
| `lan_vf_bdf` | string | Required when `backend=sriov` |

### IF-CFG-GUESTS

| Key | Type | Description |
|-----|------|-------------|
| `wan_memory_mb`, `lan_memory_mb` | integer | Private guest memory sizes |
| `wan_vcpus`, `lan_vcpus` | integer | Demonstrator uses 1 each |

### IF-CFG-RING

| Key | Type | Description |
|-----|------|-------------|
| `capacity_pow2` | integer | Ring slot count (power of two) |
| `frame_capacity` | integer | Max frame length (bytes) |

### IF-CFG-BENCH

| Key | Type | Description |
|-----|------|-------------|
| `frame_size` | integer | Bench frame size (demonstrator: 64; airborne worst-case stress) |
| `duration_secs` | integer | Measurement duration (must be &gt; 0) |
| `bidirectional` | bool | Full-duplex when true |
| `target_gbps` | float | Objective threshold for `target_met` |
| `report_path` | path | Optional; default `target/bench-report.json` |

### IF-RPT-BENCH — Bench report JSON (normative fields)

Published by `cargo xtask bench` (path from `bench.report_path` or `HV_BENCH_REPORT`). Required for SYS-DP-007 / SWR-HVN-058:

| Field | Type | Description |
|-------|------|-------------|
| `frame_size` | integer | Bytes |
| `duration_secs` | number | Actual wall-clock duration |
| `bidirectional` | bool | As run |
| `backend` | string | `sriov` \| `inject` (actual backend after open policy) |
| `gbps` | number | **Aggregate bidirectional** wire rate from delivered TX bytes (`bytes_wan_tx + bytes_lan_tx`) × 8 / duration / 1e9 |
| `pps` | number | Aggregate delivered pps (`wan_tx + lan_tx`) / duration |
| `loss` | number | Ratio `ring_drop_full / (ring_drop_full + ring_fwd_*)` |
| `target_gbps` | number | From config |
| `target_met` | bool | `gbps >= target_gbps` — **must not** be forced true |

Optional but recommended for airborne/safety review: `environment_class`, `wan_vf_bdf`, `lan_vf_bdf`, `git_sha`, `timestamp`, `gbps_basis`, `loss_unit`.

### IF-CFG-PATHS

| Key | Type | Description |
|-----|------|-------------|
| `hv_image` | path | Hypervisor image |
| `guest_wan_image` | path | WAN guest image |
| `guest_lan_image` | path | LAN guest image |

### IF-CFG-ENV

| Variable | Effect |
|----------|--------|
| `HV_DEMO_CONFIG` | Path to TOML |
| `HV_VF_BACKEND` | Overrides `datapath.backend` |
| `HV_WAN_VF_BDF` | Overrides `datapath.wan_vf_bdf` |
| `HV_LAN_VF_BDF` | Overrides `datapath.lan_vf_bdf` |
| `HV_BENCH_REPORT` | Overrides `bench.report_path` for the JSON output |

### IF-CFG-REJECT

Parser **shall** fail closed if any mandatory field is missing or inconsistent with backend rules (e.g. `sriov` without BDFs).

---

## 9. Serial smoke protocol (`IF-SER-*`)

### IF-SER-001

On successful smoke datapath completion, the system emits ASCII token **`SMOKE_OK`** on the UART/serial stream consumed by `cargo xtask smoke`.

### IF-SER-002

Multi-line structured serial JSON is **N/A for demonstrator**; bench uses a file report.

---

## 10. Sequence charts

### 10.1 Smoke (Inject path)

```text
Operator          xtask           QEMU/hv         WAN           Ring          LAN
   |                |                |             |             |             |
   |-- smoke ------->|                |             |             |             |
   |                |-- start ------->|             |             |             |
   |                |                |-- boot+map ->|             |             |
   |                |                |------------->|-------------|------------>|
   |                |                |             |<- inject RX -|             |
   |                |                |             |-- try_push ->|             |
   |                |                |             |             |-- try_pop ->|
   |                |                |             |             |             |-- TX
   |                |                |             |             |             |-- stats
   |                |<- SMOKE_OK ----|<------------ UART -------------------   |
   |<- Pass --------|                |             |             |             |
```

### 10.2 Configuration fail-closed

```text
Operator    demo-config
   |             |
   |-- load ---->|
   |             |-- missing field?
   |<- error ----|
   |  (no QEMU)
```

---

## 11. Interface index

| ID | Summary | Normative depth |
|----|---------|-----------------|
| IF-RING-001 | FrameDesc packing | Binary |
| IF-RING-002 | Header + slots layout | Binary |
| IF-RING-003 | push/pop semantics | Behaviour |
| IF-RING-004 | Payload arena | Behaviour |
| IF-RING-005 | Init errors | Behaviour |
| IF-RING-006 | Bidirectional realisation | Behaviour |
| IF-NIC-001 | NicBackend methods | Logical API |
| IF-NIC-002 | Backend types | Taxonomy |
| IF-NIC-003 | Open policy | Behaviour |
| IF-NIC-004 | Error classes | Behaviour |
| IF-BOOT-001..004 | Boot blob fields | Semantic (packing later) |
| IF-UART-001..002 | UART path | Behaviour |
| IF-STATS-001..003 | Counters + packing | Semantic + layout |
| IF-CFG-* | TOML + env | Schema |
| IF-SER-001..002 | Smoke token | Protocol |
| IF-RPT-BENCH | Bench JSON fields | Schema |

---

## 12. Change record

| Issue | Date | Description |
|-------|------|-------------|
| 0.1 | 2026-10-26 | Skeleton seed |
| 0.2 | 2026-10-26 | Normative ring packing; open policy tables; sequences; maturity matrix |
| 0.3 | 2026-10-26 | Partitioning coupling note; IF-RPT-BENCH for airborne/safety rate evidence |
| 0.4 | 2026-10-26 | IF-STATS-003 packing; bench `report_path` / `HV_BENCH_REPORT`; gbps/loss units |
