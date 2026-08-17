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
| Boot ABI layout | UNIT + REVIEW |
| Guest ABI layout | UNIT + REVIEW |
| No-panic policy (production code) | REVIEW + CI |
| ObservedPlatform | QEMU + REAL_HW (future) |
| VMX / EPT / VT-d hardware | QEMU + REAL_HW (future) |
| 200 Mbit/s datapath | PERFORMANCE (future) |

QEMU is not sufficient proof for silicon-specific properties.
