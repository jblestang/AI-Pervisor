//! ACPI discovery numeric constants.

/// ACPI common table header length in bytes.
pub const ACPI_TABLE_HEADER_LENGTH: usize = 36;

/// XSDT table signature bytes.
pub const XSDT_SIGNATURE: [u8; 4] = *b"XSDT";

/// RSDT table signature bytes.
pub const RSDT_SIGNATURE: [u8; 4] = *b"RSDT";

/// Size of one XSDT entry pointer.
pub const XSDT_ENTRY_SIZE: usize = 8;

/// Size of one RSDT entry pointer.
pub const RSDT_ENTRY_SIZE: usize = 4;

/// Maximum declared length for a single ACPI table during discovery.
pub const ACPI_TABLE_MAX_LENGTH: usize = 1 << 20;

/// Maximum ACPI table pointers processed from one XSDT/RSDT root table.
pub const ACPI_ROOT_MAX_ENTRIES: usize = 256;

/// Maximum total bytes collected across all ACPI tables in one walk.
pub const ACPI_COLLECTED_MAX_BYTES: usize = 16 << 20;
