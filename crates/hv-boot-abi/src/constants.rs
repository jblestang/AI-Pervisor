//! Firmware and boot-time numeric constants.

/// UEFI memory type for conventional RAM.
pub const EFI_MEMORY_CONVENTIONAL: u32 = 7;

/// UEFI firmware page size in bytes.
pub const UEFI_PAGE_SIZE: u64 = 4096;

/// Minimum UEFI memory descriptor size per the spec (40 bytes on x86-64).
pub const UEFI_MEMORY_DESCRIPTOR_MIN_SIZE: usize = 40;

/// OVMF memory map descriptor stride on x86-64.
pub const UEFI_MEMORY_DESCRIPTOR_OVMF_SIZE: usize = 48;

/// ACPI RSDP signature bytes (`RSD PTR `).
pub const RSDP_SIGNATURE: [u8; 8] = *b"RSD PTR ";

/// ACPI 1.0 RSDP checksum coverage length in bytes.
pub const RSDP_V1_CHECKSUM_LENGTH: usize = 20;

/// ACPI 2.0+ RSDP revision threshold.
pub const RSDP_REVISION_ACPI2: u8 = 2;

/// ACPI DMAR table signature.
pub const DMAR_SIGNATURE: [u8; 4] = *b"DMAR";

/// Byte offset of the DMAR host address width field from the table base.
pub const DMAR_HOST_ADDRESS_WIDTH_OFFSET: usize = 0x24;

/// Byte offset of the DMAR flags field from the table base.
pub const DMAR_FLAGS_OFFSET: usize = 0x25;

/// Minimum valid DMAR table length in bytes.
pub const DMAR_MIN_LENGTH: usize = 0x30;

/// DMAR flags bit indicating interrupt remapping support.
pub const DMAR_FLAG_INTR_REMAP: u8 = 0x01;

/// Serial log marker emitted after successful REAL_HW Gate C init.
pub const REAL_HW_BOOT_SUCCESS_MARKER: &str = "hypervisor Gate C REAL_HW boot succeeded";
/// Serial log marker emitted when VMXON executes under REAL_HW Gate C.
pub const REAL_HW_VMXON_EXECUTED_MARKER: &str = "REAL_HW: VMXON Executed";
/// Serial log marker emitted when EPT pointer load executes under REAL_HW Gate C.
pub const REAL_HW_EPT_EXECUTED_MARKER: &str = "REAL_HW: EPT pointer Executed";
/// Serial log marker emitted when VMLAUNCH executes under REAL_HW Gate C.
pub const REAL_HW_VMLAUNCH_EXECUTED_MARKER: &str = "REAL_HW: VMLAUNCH Executed";
/// Serial log marker emitted after successful Gate D datapath foundation init.
pub const GATE_D_DATAPATH_FOUNDATION_MARKER: &str =
    "hypervisor Gate D datapath foundation succeeded";
/// Serial log marker emitted when guest boot info is built for all partitions.
pub const GATE_D_BOOT_INFO_BUILT_MARKER: &str = "Gate D: guest boot info built for all partitions";
/// Serial log marker emitted after successful Gate D datapath live init.
pub const GATE_D_DATAPATH_LIVE_MARKER: &str = "hypervisor Gate D datapath live succeeded";
/// Serial log marker emitted when a synthetic frame is forwarded in→mid→out.
pub const GATE_D_IPC_FORWARD_MARKER: &str = "Gate D: IPC frame forwarded in→mid→out";
/// Serial log marker emitted when e1000 MMIO is handled on the live datapath path.
pub const GATE_D_E1000_MMIO_MARKER: &str = "Gate D: e1000 MMIO handled";
/// Serial log marker emitted after successful Gate D datapath malicious init.
pub const GATE_D_DATAPATH_MALICIOUS_MARKER: &str = "hypervisor Gate D datapath malicious succeeded";
/// Serial log marker emitted when IPC integrity checks pass on the malicious datapath path.
pub const GATE_D_IPC_INTEGRITY_MARKER: &str = "Gate D: IPC integrity checks passed";
/// Serial log marker emitted after successful Gate D datapath guests init.
pub const GATE_D_DATAPATH_GUESTS_MARKER: &str = "hypervisor Gate D datapath guests succeeded";
/// Serial log marker emitted when guest ELF images are installed for all partitions.
pub const GATE_D_GUEST_ELF_INSTALLED_MARKER: &str =
    "Gate D: guest ELF installed for all partitions";
/// Serial log marker emitted when multi-partition VMLAUNCH seams complete.
pub const GATE_D_MULTI_VMLAUNCH_MARKER: &str = "Gate D: multi-partition VMLAUNCH executed";
/// Serial log marker emitted after successful Gate D datapath benchmark init.
pub const GATE_D_DATAPATH_BENCHMARK_MARKER: &str = "hypervisor Gate D datapath benchmark succeeded";
/// Serial log marker emitted when the 200 Mbit/s benchmark target is met.
pub const GATE_D_BENCHMARK_TARGET_MET_MARKER: &str = "Gate D: benchmark target 200 Mbit/s met";
/// Serial log marker emitted after successful Gate D datapath runtime init.
pub const GATE_D_DATAPATH_RUNTIME_MARKER: &str = "hypervisor Gate D datapath runtime succeeded";
/// Serial log marker emitted when a guest-driven frame traverses in→mid→out under VMX.
pub const GATE_D_GUEST_DATAPATH_FRAME_MARKER: &str =
    "Gate D: guest datapath frame forwarded in→mid→out";
/// Gate D guest source-tree ELF install succeeded for all partitions.
pub const GATE_D_GUEST_SOURCE_ELF_MARKER: &str =
    "Gate D: guest source ELF installed for all partitions";
/// Gate D guest boot info installed and RDI patched for all partitions.
pub const GATE_D_GUEST_BOOT_INFO_INSTALLED_MARKER: &str =
    "Gate D: guest boot info installed for all partitions";
/// Gate D live VMX guest code execution attempted for all source-tree partitions.
pub const GATE_D_GUEST_EXECUTION_MARKER: &str =
    "Gate D: guest source-tree code executed under VMX for all partitions";
/// Gate D in-VM guest throughput benchmark orchestration succeeded.
pub const GATE_D_GUEST_THROUGHPUT_MARKER: &str =
    "hypervisor Gate D datapath guest throughput succeeded";
/// Gate D in-VM guest throughput met the 200 Mbit/s target.
pub const GATE_D_GUEST_THROUGHPUT_TARGET_MET_MARKER: &str =
    "Gate D: guest throughput target 200 Mbit/s met";
/// Gate D in-VM guest throughput measured under live VMX.
pub const GATE_D_GUEST_THROUGHPUT_EXECUTED_MARKER: &str =
    "Gate D: guest throughput measured under live VMX";
