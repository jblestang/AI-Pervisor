# Proof levels

Each critical requirement declares its validation level using these categories:

- `UNIT`
- `PROPERTY`
- `FUZZ`
- `MIRI`
- `MODEL`
- `QEMU`
- `REAL_HW`
- `PERFORMANCE`
- `REVIEW`

## Initial matrix (Phases 0–4)

| Requirement | Levels |
|-------------|--------|
| Newtypes / safe arithmetic | UNIT + PROPERTY + MIRI |
| YAML syntax / semantic validation | UNIT + PROPERTY + FUZZ |
| Normalization determinism | UNIT |
| Configuration hash | UNIT |
| IPC DAG / unidirectional policy | UNIT + PROPERTY |
| PCI unique ownership | UNIT + PROPERTY |
| PlatformRequirements extraction | UNIT |
| StaticIntentIR determinism | UNIT |
| ObservedPlatform validation | UNIT |
| StaticPlatformIR planning | UNIT |
| Boot ABI layout | UNIT + FUZZ + REVIEW |
| Guest ABI layout | UNIT + REVIEW |
| Boot-path ACPI / UEFI parsing | UNIT + FUZZ |
| Observed platform JSON | UNIT + FUZZ |
| No-panic policy (production code) | REVIEW + CI + FUZZ (parsers) |
| ObservedPlatform | QEMU + REAL_HW (future) |
| VMX / EPT / VT-d init planning | UNIT + REVIEW (Phase 9–10 mock backends) |
| VMX / EPT / VT-d structure programming | UNIT + REVIEW (Phase 12 programming backends) |
| VMX / EPT / VT-d CPU instruction seams | UNIT + REVIEW (Phase 13 validate-only seams) |
| VMX / EPT / VT-d live privileged instructions | REVIEW + REAL_HW (Phase 14 modules; CI validate-only fallback) |
| REAL_HW resident install + VMCS prepare | UNIT + REVIEW + REAL_HW (Phase 15) |
| VMX launch (VMCS fields + VMLAUNCH) | UNIT + REVIEW + REAL_HW (Phase 16) |
| Guest datapath smoke | UNIT + REVIEW (Phase 18: synthetic in→mid→out IPC forward + e1000 doorbell) |
| IPC forwarding integrity | UNIT (Phase 18 enqueue bounds; Phase 19 compromised-guest integrity scanner) |
| Guest ELF install + multi-partition launch | UNIT + REVIEW (Phase 20: reference ELF64 stubs + validate-only VMLAUNCH seams) |
| Datapath throughput benchmark | UNIT + PERFORMANCE (Phase 21: mock + host wall-clock benchmark per `docs/benchmark.md`) |
| Guest datapath runtime under VMX | UNIT + REVIEW (Phase 22: guest-role IPC/e1000 hops + multi-partition VM-exit seam validation) |

QEMU is not sufficient proof for silicon-specific properties.
