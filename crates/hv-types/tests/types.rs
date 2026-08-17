//! hv-types extended unit tests.

use hv_types::{
    align_down, align_up, checked_add_usize, checked_mul_usize, is_aligned, ApicId, ArithmeticError,
    ByteSize, Gibibyte, GuestPhysAddr, HostPhysAddr, IommuDomainId, IpcChannelId, LogicalCpuId,
    PackageId, PageCount, PciBdf, PciBus, PciDevice, PciFunction, PciSegment, PhysicalCoreId,
    VcpuId, VmId,
};

#[test]
fn newtype_accessors_cover_id_and_addr_types() {
    assert_eq!(VmId::new(3).raw(), 3);
    assert_eq!(IpcChannelId::new(7).raw(), 7);
    assert_eq!(VcpuId::new(1).raw(), 1);
    assert_eq!(LogicalCpuId::new(2).raw(), 2);
    assert_eq!(PhysicalCoreId::new(4).raw(), 4);
    assert_eq!(PackageId::new(5).raw(), 5);
    assert_eq!(IommuDomainId::new(6).raw(), 6);
    assert_eq!(ApicId::new(8).raw(), 8);
    assert_eq!(HostPhysAddr::new(9).raw(), 9);
    assert_eq!(GuestPhysAddr::new(10).raw(), 10);
}

#[test]
fn pci_newtypes_and_bdf() {
    let bdf = PciBdf::new(
        PciSegment::new(1),
        PciBus::new(2),
        PciDevice::new(3),
        PciFunction::new(4),
    );
    assert_eq!(bdf.segment.raw(), 1);
    assert_eq!(bdf.bus.raw(), 2);
    assert_eq!(bdf.device.raw(), 3);
    assert_eq!(bdf.function.raw(), 4);
}

#[test]
fn byte_size_returns_none_when_too_large_for_usize() {
    if usize::MAX as u64 == u64::MAX {
        return;
    }
    assert_eq!(ByteSize::new(usize::MAX as u64 + 1).as_usize(), None);
}

#[test]
fn page_count_accessor() {
    assert_eq!(PageCount::new(8).pages(), 8);
}

#[test]
fn gibibyte_overflow_is_rejected() {
    let err = Gibibyte::new(u64::MAX).to_bytes();
    assert_eq!(err, Err(ArithmeticError::MulOverflow));
}

#[test]
fn arithmetic_error_paths() {
    assert_eq!(align_down(5, 0), Err(ArithmeticError::InvalidAlignment));
    assert_eq!(is_aligned(1, 3), Err(ArithmeticError::InvalidAlignment));
    assert_eq!(
        checked_add_usize(usize::MAX, 1),
        Err(ArithmeticError::AddOverflow)
    );
    assert_eq!(
        checked_mul_usize(usize::MAX, 2),
        Err(ArithmeticError::MulOverflow)
    );
    assert_eq!(align_up(usize::MAX, 4096), Err(ArithmeticError::AddOverflow));
}
