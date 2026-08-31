# Phase 28 expert review

Multi-domain review of KVM Gate D smoke integration: live boot-chain guest embedding and Gate D relay-live serial marker verification (`cursor/phase-28-kvm-gate-d-smoke-0b4f`).

## Domains reviewed

| Domain | Scope |
|--------|--------|
| Live boot chain | `run_build_boot_chain_live` invokes `build-guests` before hypervisor EFI build |
| KVM smoke harness | `evaluate_gate_d_guest_relay_live_smoke`, updated `live-qemu-smoke` success criteria |
| Docs | `ovmf-boot.md`, `architecture.md`, `platform-contract.md` |

## Phase 27 deferrals closed

| Phase 27 item | Phase 28 disposition |
|---------------|---------------------|
| `build-guests` not in live boot-chain pipeline | **Closed** — `build-boot-chain-live` runs `build-guests` first |
| `live-qemu-smoke` checks wrong success marker | **Closed** — accepts Gate D throughput markers or legacy REAL_HW marker |
| No automated Gate D marker smoke | **Closed** — smoke requires source ELF, boot-info, throughput target, and REAL_HW VMX marker |
| In-VM relay frame measurement | **Deferred** — smoke does not require `GATE_D_GUEST_THROUGHPUT_EXECUTED` or guest relay-complete marker |

## Feature matrix

| Component | Change |
|-----------|--------|
| `cargo xtask build-boot-chain-live` | Runs `build-guests` then builds REAL_HW relay-live hypervisor |
| `cargo xtask live-qemu-smoke` | Validates Gate D guest relay live serial markers under KVM |

## Serial markers required by smoke

- `Gate D: guest source ELF installed for all partitions`
- `Gate D: guest boot info installed for all partitions`
- `Gate D: guest throughput target 200 Mbit/s met`
- `hypervisor Gate D datapath guest throughput succeeded`
- At least one of: `REAL_HW: VMXON Executed`, `REAL_HW: EPT pointer Executed`, `REAL_HW: VMLAUNCH Executed`

Legacy fallback: `hypervisor Gate C REAL_HW boot succeeded` (pre–Gate D throughput firmware).

## Verification

- `cargo test -p xtask live_qemu`
- `cargo xtask build-boot-chain-live` (requires `x86_64-unknown-none` + UEFI toolchain)
- `cargo xtask live-qemu-smoke` — pass or skip (exit 0) when KVM/VMX unavailable

## Review status

Phase 28 makes KVM Gate D guest relay live smoke reliable: the live boot chain embeds source-tree guests, and `live-qemu-smoke` validates Gate D orchestration markers instead of the legacy Gate C-only success string. Live in-VM guest execution and throughput `Executed` disposition remain deferred to a future phase.
