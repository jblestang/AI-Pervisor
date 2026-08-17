//! Identifier newtypes.

macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct $name(pub u32);

        impl $name {
            /// Creates a new identifier from a raw value.
            pub const fn new(raw: u32) -> Self {
                Self(raw)
            }

            /// Returns the raw identifier value.
            pub const fn raw(self) -> u32 {
                self.0
            }
        }
    };
}

macro_rules! define_addr {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct $name(pub u64);

        impl $name {
            /// Creates a new address from a raw value.
            pub const fn new(raw: u64) -> Self {
                Self(raw)
            }

            /// Returns the raw address value.
            pub const fn raw(self) -> u64 {
                self.0
            }
        }
    };
}

define_id!(/// Virtual machine identifier assigned deterministically from configuration.
    VmId);
define_id!(/// IPC channel identifier assigned deterministically from configuration.
    IpcChannelId);
define_id!(/// Virtual CPU identifier within a partition.
    VcpuId);
define_id!(/// Logical CPU index as seen by the platform.
    LogicalCpuId);
define_id!(/// Physical core identifier.
    PhysicalCoreId);
define_id!(/// CPU package identifier.
    PackageId);
define_id!(/// IOMMU domain identifier.
    IommuDomainId);
define_id!(/// APIC identifier.
    ApicId);
define_id!(/// Interrupt vector number.
    InterruptVector);

define_addr!(/// Host physical address.
    HostPhysAddr);
define_addr!(/// Guest physical address.
    GuestPhysAddr);
define_addr!(/// Host virtual address.
    HostVirtAddr);
define_addr!(/// Guest virtual address.
    GuestVirtAddr);
define_addr!(/// IO virtual address used by DMA.
    Iova);

/// PCI segment number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PciSegment(pub u16);

impl PciSegment {
    /// Creates a new PCI segment.
    pub const fn new(raw: u16) -> Self {
        Self(raw)
    }

    /// Returns the raw segment value.
    pub const fn raw(self) -> u16 {
        self.0
    }
}

/// PCI bus number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PciBus(pub u8);

impl PciBus {
    /// Creates a new PCI bus number.
    pub const fn new(raw: u8) -> Self {
        Self(raw)
    }

    /// Returns the raw bus number.
    pub const fn raw(self) -> u8 {
        self.0
    }
}

/// PCI device number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PciDevice(pub u8);

impl PciDevice {
    /// Creates a new PCI device number.
    pub const fn new(raw: u8) -> Self {
        Self(raw)
    }

    /// Returns the raw device number.
    pub const fn raw(self) -> u8 {
        self.0
    }
}

/// PCI function number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PciFunction(pub u8);

impl PciFunction {
    /// Creates a new PCI function number.
    pub const fn new(raw: u8) -> Self {
        Self(raw)
    }

    /// Returns the raw function number.
    pub const fn raw(self) -> u8 {
        self.0
    }
}

/// PCI Bus:Device.Function address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PciBdf {
    /// PCI segment.
    pub segment: PciSegment,
    /// PCI bus.
    pub bus: PciBus,
    /// PCI device.
    pub device: PciDevice,
    /// PCI function.
    pub function: PciFunction,
}

impl PciBdf {
    /// Creates a PCI BDF from its components.
    pub const fn new(
        segment: PciSegment,
        bus: PciBus,
        device: PciDevice,
        function: PciFunction,
    ) -> Self {
        Self {
            segment,
            bus,
            device,
            function,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vm_id_ordering_is_stable() {
        let a = VmId::new(0);
        let b = VmId::new(1);
        assert!(a < b);
    }

    #[test]
    fn pci_bdf_equality() {
        let bdf = PciBdf::new(
            PciSegment::new(0),
            PciBus::new(0),
            PciDevice::new(3),
            PciFunction::new(0),
        );
        assert_eq!(bdf.segment.raw(), 0);
    }
}
