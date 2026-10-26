# Lab harness notes

Issue: 0.1 — seeded by Task 6; **Task 7** owns `run-nested` / `smoke` wiring.

## What Task 6 delivered (host-testable)

| Piece | Command / artefact |
|-------|-------------------|
| EPT intent + layout from config | `cargo test -p hv --features std` |
| Freestanding `hv` image | `cargo xtask build` → `target/x86_64-unknown-none/debug/hv` |
| Flat config lift | `FlatHvConfig::from_demo(&DemoConfig)` (`hv` feature `std`) |
| Boot blob | `fill_boot_info` → `guest_common::BootInfo` GPAs from layout |

## Nested QEMU requirements (Task 7 must wire)

1. **Host CPU / KVM**
   - Intel VT‑x with **nested** virtualization (`kvm_intel nested=1`).
   - Confirm: `/sys/module/kvm_intel/parameters/nested` == `1`.
   - Cloud VMs without KVM or nested VMX **cannot** smoke Type‑1 `hv` — use host unit tests only.

2. **QEMU argv (from `DemoConfig` only)**
   - `-enable-kvm`, CPU with `+vmx` (e.g. `-cpu host,+vmx`).
   - `-m` / `-smp` from `[lab]`.
   - `-device intel-iommu` when `lab.enable_iommu`.
   - Serial: `-serial stdio` (or multiplex) so UART exits are observable.
   - **Do not** hardcode BDFs / memory in `xtask`; read `config/demo.toml`.

3. **Loader ABI for `hv`**
   - Pass `*const FlatHvConfig` in `rdi` at `_start` (SysV).
   - Build `FlatHvConfig` via `FlatHvConfig::from_demo`.
   - Load `paths.guest_wan_image` / `paths.guest_lan_image` into buffers mapped at `GuestLayout` private GPAs (EPT identity or explicit HPA).
   - Write `BootInfo` pages at `boot_info_{wan,lan}` before `VMLAUNCH`.
   - Install EPT tables from `identity_entries` / map entries; `vmwrite` EPTP (TCB-EPT-001).

4. **Smoke token**
   - Guests emit UART logs; LAN path eventually prints `SMOKE_OK` (ICD `IF-SER-*`) once datapath is live — Task 7 `smoke` waits for that string.

5. **SR‑IOV / VFIO** (optional until Task 10)
   - Bind `datapath.wan_vf_bdf` / `lan_vf_bdf` to `vfio-pci` when `backend=sriov`.
   - Pass devices into QEMU; fail fast if missing and `allow_inject_fallback=false`.

## Cloud agent limitation

This environment typically **lacks** nested KVM. Task 6 verification is therefore:

```bash
cargo test -p hv --features std
cargo xtask build
```

Full `run-nested` / `smoke` is **deferred to Task 7** on a lab machine with nested VT‑x.

## Config tips (`[hv]` section)

| Key | Role |
|-----|------|
| `gpa_window_base` | Start of packed guest GPA layout |
| `uart_io_port` | I/O tip for UART exits / boot info |
