# Lab harness notes

Issue: 0.2 — Task 7 smoke, Task 9 `xtask bench` / stats ABI, **Task 10 SR‑IOV lab path**.

## Commands

| Command | Behaviour |
|---------|-----------|
| `cargo xtask build` | Host libs + freestanding `hv` / `guest-wan` / `guest-lan` |
| `cargo xtask run-nested` | QEMU argv **only** from `DemoConfig`; Multiboot1 `-kernel` + `-initrd` modules |
| `cargo xtask smoke` | Same boot path; wait for serial ASCII token `SMOKE_OK` (`IF-SER-001`) — **Inject only** until L2 VMLAUNCH (see below) |
| `cargo xtask bench` | Timed E2E measure from `[bench]`; writes `IF-RPT-BENCH` JSON (`target_met` may be false; exit 0 if measured) |

Config resolution: `HV_DEMO_CONFIG` → `config/demo.toml` → `config/demo.example.toml`.

Env overrides: `HV_VF_BACKEND`, `HV_WAN_VF_BDF`, `HV_LAN_VF_BDF`, `HV_BENCH_REPORT` (report path).

## What Task 6/7/9/10 delivered

| Piece | Command / artefact |
|-------|-------------------|
| EPT intent + layout from config | `cargo test -p hv --features std` |
| Freestanding Multiboot1 `hv` | `cargo xtask build` → ELF + xtask `objcopy` → `hv.bin` (QEMU rejects ELF64 Multiboot) |
| Flat config blob | Multiboot module 0 = `FlatHvConfigBlob` (`FLAT` magic) |
| Guest images | Multiboot modules 1/2 = `guest-wan` / `guest-lan` |
| L1 inject smoke | When **resolved** backend is Inject, `hv` runs the real `guest_common` WAN→LAN forward and prints `SMOKE_OK` |
| Sriov open policy | `pci-vf` + xtask: VF must exist **and** be bound to `vfio-pci`; no silent Inject when `allow_inject_fallback=false` |
| L2 VMLAUNCH | Scaffolded (module load + EPT pack + VMXON); full VMCS host-state / VMLAUNCH still incomplete — UART says so |

## Nested QEMU requirements

1. **Host CPU / KVM**
   - Intel VT‑x with **nested** virtualization (`kvm_intel nested=1`).
   - Confirm: `/sys/module/kvm_intel/parameters/nested` ∈ `{Y,1}`.
   - Confirm: `/dev/kvm` readable/writable by the operator.
   - Cloud VMs without KVM or nested VMX **cannot** smoke Type‑1 `hv` — use host unit tests only.

2. **QEMU argv (from `DemoConfig` only — `xtask/src/qemu.rs`)**
   - Accel: default **TCG** Multiboot boot (`HV_QEMU_ACCEL=tcg`); set `HV_QEMU_ACCEL=kvm` for `-accel kvm -cpu host,+vmx` (Multiboot+KVM entry is flaky on some hosts — see below)
   - `-m` / `-smp` from `[lab]`
   - `-device intel-iommu,…` when `lab.enable_iommu`
   - `-device vfio-pci,host=<BDF>` for each of `datapath.wan_vf_bdf` / `lan_vf_bdf` when `backend=sriov`
   - `-serial stdio` + `-display none` (not `-nographic`, which double-binds stdio)
   - `-kernel` = `paths.hv_image`, `-initrd` = `flat.bin,guest-wan,guest-lan`

3. **Loader ABI for `hv`**
   - Multiboot1 + `HAS_ADDR` trampoline (`hv/src/boot.s`) → raw `hv.bin` → `rust_entry(info)`
   - Module 0: `FlatHvConfigBlob` written by xtask from **resolved** `DemoConfig` (no lab literals in `hv`)
   - Modules 1/2: guest ELF/raw images; `load_guest_image` copies into private buffers
   - BootInfo fill + EPT intent/pack; VMXON when nested VT‑x works
   - **Smoke token:** L1 inject path emits `SMOKE_OK` when `nic_backend_inject`; LAN guest also links `emit_smoke_ok` for future L2

4. **Inject vs Sriov policy (smoke / run-nested / bench)**

   | Config | VF sysfs + `vfio-pci` bind | Result |
   |--------|------------------------------|--------|
   | `backend=inject` | N/A | Inject |
   | `backend=sriov` | both WAN+LAN ready | **Sriov** (no rewrite) |
   | `backend=sriov` + `allow_inject_fallback=true` | missing / unbound | Inject + **stderr note** |
   | `backend=sriov` + `allow_inject_fallback=false` | missing / unbound | **fail fast** (actionable error; no silent mock) |

   Flat blob / BootInfo `nic_backend` follows the **resolved** backend only. `allow_inject_fallback` never marks Sriov as Inject while VFs remain selected (SWR-NIC-023).

## VFIO / SR‑IOV bind checklist (Task 10)

Complete every box before claiming a Sriov smoke/bench run.

### A. Platform (IOMMU + nested)

- [ ] Intel IOMMU enabled in firmware; kernel cmdline includes `intel_iommu=on` (verify: `dmesg \| grep -i iommu` shows DMAR / IOMMU enabled)
- [ ] `lab.enable_iommu = true` so xtask passes `-device intel-iommu,intremap=on,device-iotlb=on`
- [ ] `kvm_intel nested=1` loaded (`cat /sys/module/kvm_intel/parameters/nested` ∈ `Y`/`1`)
- [ ] Operator can open `/dev/kvm`
- [ ] QEMU binary matches `lab.qemu_bin` (default `qemu-system-x86_64`)
- [ ] For real nested VT‑x (not just TCG Multiboot): `HV_QEMU_ACCEL=kvm` once Multiboot+KVM entry is known-good on the lab image

### B. Create / locate VFs

- [ ] Identify PF: `lspci -nn \| grep -i ether` (record PF BDF)
- [ ] Enable VFs: `echo N \| sudo tee /sys/bus/pci/devices/<PF>/sriov_numvfs`
- [ ] Note VF BDFs: `lspci -d ::0200` / `ls /sys/bus/pci/devices/<PF>/virtfn*`
- [ ] Put exact VF BDFs into `config/demo.toml` as `datapath.wan_vf_bdf` / `lan_vf_bdf` (or `HV_WAN_VF_BDF` / `HV_LAN_VF_BDF`)
- [ ] Confirm IOMMU groups: `readlink /sys/bus/pci/devices/<VF>/iommu_group` (each VF should be isolatable; ACS quirks may block)

### C. Bind VFs to `vfio-pci`

```bash
# Example — replace BDFs with values from your demo.toml (no hardcoding in repo tools).
VF=0000:01:10.0
# Unbind existing driver if any:
if [ -e /sys/bus/pci/devices/$VF/driver ]; then
  echo $VF | sudo tee /sys/bus/pci/devices/$VF/driver/unbind
fi
echo vfio-pci | sudo tee /sys/bus/pci/devices/$VF/driver_override
sudo bash -c "echo $VF > /sys/bus/pci/drivers/vfio-pci/bind" || \
  sudo bash -c "echo $(cat /sys/bus/pci/devices/$VF/vendor) $(cat /sys/bus/pci/devices/$VF/device) > /sys/bus/pci/drivers/vfio-pci/new_id"
# Verify bind (xtask + pci-vf both require this leaf name):
basename "$(readlink -f /sys/bus/pci/devices/$VF/driver)"   # → vfio-pci
ls -l /dev/vfio/
```

- [ ] Repeat for the LAN VF BDF
- [ ] Confirm `driver` symlink under each VF sysfs dir resolves to `vfio-pci`
- [ ] `ls /dev/vfio/<group>` exists for each VF’s IOMMU group

### D. Config for official Sriov

```toml
[datapath]
backend = "sriov"
allow_inject_fallback = false
wan_vf_bdf = "<your-wan-vf>"
lan_vf_bdf = "<your-lan-vf>"
```

Guests **must not** be rebuilt when flipping `backend` / BDFs — only this config (or env overrides). `BootInfo.nic_backend` carries the selection; both `InjectNic` and `SriovNic` stay linked in guest images.

### E. Launch (when VFs ready)

```bash
export HV_DEMO_CONFIG=config/demo.toml   # backend=sriov, BDFs set, allow_inject_fallback=false
cargo xtask smoke    # today: fail-fast with L2 gap message (see remaining gaps)
cargo xtask bench    # today: fail-fast with L2 gap message
cargo xtask run-nested  # boots QEMU with vfio-pci devices attached; observe UART
```

- [ ] If bind/IOMMU wrong: xtask fails at resolve with “not bound to vfio-pci” / “sysfs node missing” — fix bind, do **not** flip to Inject for official STR figures
- [ ] If VFs ready but smoke/bench refuse: expected until remaining gaps below are closed

### F. Degraded lab (no VF) — Inject only

```bash
# Either edit demo.toml:
#   backend = "inject"
#   allow_inject_fallback = true
# Or:
HV_VF_BACKEND=inject cargo xtask smoke
HV_DEMO_CONFIG=config/bench.inject.ci.toml cargo xtask bench
```

Mark STR/evidence as **Inject / degraded**, never as Sriov.

## Sriov smoke / remaining gaps (honest E2E status)

`backend=sriov` with both VFs **vfio-bound** selects Sriov correctly and attaches `-device vfio-pci,host=…`. Full Sriov E2E (`SMOKE_OK` from LAN guest + 1 Gbps@64B Sriov report) is **blocked** by the following concrete gaps:

| # | Gap | Where | Blocks |
|---|-----|-------|--------|
| 1 | **VMCS host-state incomplete** — no successful `VMLAUNCH` into L2 guests | `hv` VMX path (`hv: VMLAUNCH deferred…`) | Guest entry / UART from LAN |
| 2 | **Guest live poll loop** not yet wired to mapped rings/NIC MMIO (symbols retained only) | `guest-wan` / `guest-lan` | Frame forward under L2 |
| 3 | **Sriov BAR / descriptor-ring MMIO** not mapped (`SriovNic` pre-MMIO stub: RX empty, TX `IoFailure`) | `pci-vf/src/sriov.rs`, TCB-NIC-001 | Real VF datapath |
| 4 | **BootInfo `nic_mmio_gpa` / `nic_mmio_len`** still zero (hv does not program VF BARs into guests) | `hv/src/boot_blob.rs` | Guest Sriov open tips |
| 5 | **Multiboot + KVM entry flaky** on some lab images (TCG default); nested VT‑x needed for real L2 | `xtask` / `HARNESS` Multiboot note | Hardware VMXON/VMLAUNCH under KVM |
| 6 | **IOMMU group / ACS** lab-specific isolation of VFs into QEMU | Host platform | `-device vfio-pci` open at QEMU start |

Until gaps **1–4** close, `cargo xtask smoke` and `cargo xtask bench` with resolved Sriov **exit non-zero immediately** with a pointer here — they do **not** silently run Inject and relabel it as Sriov.

## Smoke serial protocol (`IF-SER-001`)

- Token: ASCII `SMOKE_OK` (optional trailing newline)
- Emitted by L1 inject smoke in `hv` after a real `forward_wan_poll` + `forward_lan_poll` round using `InjectNic` + `SpscRing` (same code guests link)
- LAN guest exports `guest_common::emit_smoke_ok` for the L2 path once VMLAUNCH completes
- `cargo xtask smoke` fails with serial dump + hints on timeout (default 30s) for Inject; Sriov fails fast (table above)

## Stats ABI (`IF-STATS-*`) — shared page for smoke/bench

Source of truth: ICD §7 + `guest_common::Stats` / `stats_off`.

| Item | Contract |
|------|----------|
| Packing | `#[repr(C)]` little-endian `u64` × 11 = **88** bytes (`STATS_PAGE_SIZE`) |
| Offsets | `wan_rx@0` … `bytes_lan_tx@80` (see ICD `IF-STATS-003`) |
| Writers | Owning guest / L1 inject path / host Inject harness (same forward code) |
| Readers | Harness may see torn 64-bit loads; bench uses start/end [`StatsSnapshot`] deltas |
| No lab constants | Frame size, duration, target Gbps, report path come from `[bench]` only |

## Bench (`IF-CFG-BENCH` / `IF-RPT-BENCH`)

### Policy (same as smoke)

- `backend=inject` → Inject measurement
- `backend=sriov` + both VF BDFs **vfio-bound** → **Sriov** selected; E2E through L2 guests is required for an honest Sriov figure (blocked until VMLAUNCH datapath is complete — tool exits non-zero rather than silently publishing Inject as Sriov)
- `backend=sriov` + VF missing/unbound + `allow_inject_fallback=true` → Inject with stderr note; report `backend=inject`, `environment_class=inject-harness`
- `backend=sriov` + VF missing/unbound + `allow_inject_fallback=false` → fail fast

### Inject measurement (current spike)

`cargo xtask bench` drives the **real** `guest_common` WAN↔LAN forward path with `InjectNic` + `SpscRing` for `bench.duration_secs` at `bench.frame_size` (bidirectional when configured). This is the same datapath code as L1 inject smoke — not a mock. Report fields:

| Field | Meaning |
|-------|---------|
| `gbps` | Aggregate bidirectional: `(bytes_wan_tx + bytes_lan_tx) * 8 / duration / 1e9` |
| `pps` | `(wan_tx + lan_tx) / duration` |
| `loss` | `ring_drop_full / (ring_drop_full + ring_fwd_wan_to_lan + ring_fwd_lan_to_wan)` |
| `target_met` | `gbps >= target_gbps` (false is **not** a tool failure) |

### CI / short sample

```bash
HV_DEMO_CONFIG=config/bench.inject.ci.toml cargo xtask bench
# → target/bench-report.json (or HV_BENCH_REPORT / bench.report_path)
```

Schema sample: `docs/system/samples/bench-report.example.json`.

## Config tips (`[hv]` section)

| Key | Role |
|-----|------|
| `gpa_window_base` | Start of packed guest GPA layout (keep inside L1 identity map for nested Multiboot trampoline — see example / local `demo.toml`) |
| `uart_io_port` | I/O tip for UART exits / boot info / platform serial |

## Multiboot + KVM note

QEMU's Multiboot1 **ADDR** loader + ELF64 is rejected; xtask therefore `objcopy`s `hv` to `hv.bin`. On this spike's lab images, **`-accel kvm` often never reaches `_start`** (empty serial / no `0xE9` debugcon), while **TCG Multiboot enters reliably**. `cargo xtask smoke` defaults to TCG so the L1 inject datapath can emit `SMOKE_OK`. `/dev/kvm` and `nested=1` are still checked as lab capability signals; force KVM with `HV_QEMU_ACCEL=kvm` when debugging nested VT‑x.

## Cloud / agent limitation

Environments without VF hardware should use Inject (config-gated). Host tests remain:

```bash
cargo test -p hv --features std
cargo test -p pci-vf --features std
cargo test -p xtask
cargo xtask build
```
