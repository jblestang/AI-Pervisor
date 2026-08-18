//! VMCS encoded field numbers and reference control values for MODEL launch bring-up.

/// Pin-based VM-execution controls.
pub const VMCS_PIN_BASED_VM_EXEC_CONTROL: u32 = 0x0000_4000;
/// Primary processor-based VM-execution controls.
pub const VMCS_CPU_BASED_VM_EXEC_CONTROL: u32 = 0x0000_4002;
/// VM-exit controls.
pub const VMCS_VM_EXIT_CONTROLS: u32 = 0x0000_400C;
/// VM-entry controls.
pub const VMCS_VM_ENTRY_CONTROLS: u32 = 0x0000_4012;
/// Secondary processor-based VM-execution controls.
pub const VMCS_SECONDARY_VM_EXEC_CONTROL: u32 = 0x0000_401E;

/// Guest CR3.
pub const VMCS_GUEST_CR3: u32 = 0x0000_6802;
/// Guest RSP.
pub const VMCS_GUEST_RSP: u32 = 0x0000_681C;
/// Guest RIP.
pub const VMCS_GUEST_RIP: u32 = 0x0000_681E;
/// Host CR3.
pub const VMCS_HOST_CR3: u32 = 0x0000_6C02;
/// Host RSP.
pub const VMCS_HOST_RSP: u32 = 0x0000_6C14;
/// Host RIP (VM-exit handler entry).
pub const VMCS_HOST_RIP: u32 = 0x0000_6C16;

/// Activate secondary controls (bit 31) in primary processor-based controls.
pub const CPU_BASED_ACTIVATE_SECONDARY_CONTROLS: u64 = 1 << 31;
/// Enable EPT (bit 1) in secondary processor-based controls.
pub const SECONDARY_ENABLE_EPT: u64 = 1 << 1;
/// Host address-space size / 64-bit host (bit 9) in VM-exit controls.
pub const VM_EXIT_HOST_ADDR_SPACE_SIZE: u64 = 1 << 9;
/// IA-32e mode guest entry (bit 9) in VM-entry controls.
pub const VM_ENTRY_IA32E_MODE: u64 = 1 << 9;

/// Byte offset of the host VM-exit stub inside the hypervisor reserve (MODEL).
pub const VMX_HOST_EXIT_STUB_OFFSET: u64 = 0x1000;
