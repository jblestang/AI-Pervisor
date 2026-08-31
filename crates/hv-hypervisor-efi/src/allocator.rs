//! UEFI page allocation for REAL_HW Gate C resident installs.

#![allow(unsafe_code)]

use hv_x86_cpu::{CpuSeamError, CpuSeamErrorKind, PageAllocator};
#[cfg(not(test))]
use uefi::boot::{self, AllocateType};
#[cfg(not(test))]
use uefi::mem::memory_map::MemoryType;

/// Allocates conventional UEFI pages and copies programmed Gate C structures.
#[derive(Default)]
pub struct UefiPageAllocator {
    pages: alloc::vec::Vec<(u64, *mut u8, usize)>,
}

impl UefiPageAllocator {
    /// Creates an empty UEFI page allocator.
    pub fn new() -> Self {
        Self::default()
    }
}

impl PageAllocator for UefiPageAllocator {
    fn allocate_pages(&mut self, size: usize, align: u64) -> Result<u64, CpuSeamError> {
        validate_allocation_request(size, align)?;
        let page_count = pages_for_bytes(size)?;
        let (host_phys, ptr, capacity) = allocate_conventional_pages(page_count, align)?;
        self.pages.push((host_phys, ptr, capacity));
        Ok(host_phys)
    }

    fn copy_to_pages(&mut self, host_phys: u64, bytes: &[u8]) -> Result<(), CpuSeamError> {
        let (_, ptr, capacity) = self
            .pages
            .iter()
            .copied()
            .find(|(phys, _, _)| *phys == host_phys)
            .ok_or_else(|| {
                CpuSeamError::new(
                    CpuSeamErrorKind::InvalidInput,
                    "copy target was not allocated by this allocator",
                )
            })?;
        if bytes.len() > capacity {
            return Err(CpuSeamError::new(
                CpuSeamErrorKind::InvalidInput,
                "copy exceeds allocated page capacity",
            ));
        }
        // SAFETY: pointer came from UEFI AllocatePages and length is bounded.
        unsafe {
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        }
        Ok(())
    }
}

fn validate_allocation_request(size: usize, align: u64) -> Result<(), CpuSeamError> {
    if size == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "UEFI allocation size must be non-zero",
        ));
    }
    if align == 0 || align & (align - 1) != 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "UEFI allocation alignment must be a power of two",
        ));
    }
    Ok(())
}

fn pages_for_bytes(len: usize) -> Result<usize, CpuSeamError> {
    let page_size = uefi::table::boot::PAGE_SIZE;
    len.checked_add(page_size - 1)
        .ok_or_else(|| {
            CpuSeamError::new(CpuSeamErrorKind::InvalidInput, "allocation size overflow")
        })?
        .checked_div(page_size)
        .ok_or_else(|| {
            CpuSeamError::new(
                CpuSeamErrorKind::InvalidInput,
                "allocation page count overflow",
            )
        })
}

#[cfg(not(test))]
fn allocate_conventional_pages(
    page_count: usize,
    align: u64,
) -> Result<(u64, *mut u8, usize), CpuSeamError> {
    let pages = boot::allocate_pages(
        AllocateType::AnyPages,
        MemoryType::RUNTIME_SERVICES_DATA,
        page_count,
    )
    .map_err(|status| {
        CpuSeamError::new(
            CpuSeamErrorKind::Unavailable,
            alloc::format!("UEFI AllocatePages failed: {status:?}"),
        )
    })?;
    let host_phys = pages.as_ptr() as u64;
    if host_phys & (align - 1) != 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::Unavailable,
            "UEFI page allocation did not satisfy alignment",
        ));
    }
    let capacity = page_count.saturating_mul(uefi::table::boot::PAGE_SIZE);
    Ok((host_phys, pages.as_ptr(), capacity))
}

#[cfg(test)]
fn allocate_conventional_pages(
    page_count: usize,
    align: u64,
) -> Result<(u64, *mut u8, usize), CpuSeamError> {
    use std::alloc::{alloc, Layout};

    let page_size = uefi::table::boot::PAGE_SIZE;
    let size = page_count.saturating_mul(page_size);
    let layout = Layout::from_size_align(size, align as usize).map_err(|_| {
        CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "host test allocation layout invalid",
        )
    })?;
    // SAFETY: layout has non-zero size and alignment.
    let ptr = unsafe { alloc(layout) };
    if ptr.is_null() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::Unavailable,
            "host test allocation failed",
        ));
    }
    let host_phys = ptr as u64;
    if host_phys & (align - 1) != 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::Unavailable,
            "host test allocation did not satisfy alignment",
        ));
    }
    Ok((host_phys, ptr, size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uefi_page_allocator_default_constructs() {
        let _ = UefiPageAllocator::default();
    }

    #[test]
    fn uefi_page_allocator_rejects_zero_size() {
        let mut allocator = UefiPageAllocator::new();
        assert!(allocator.allocate_pages(0, 4096).is_err());
    }

    #[test]
    fn uefi_page_allocator_rejects_invalid_alignment() {
        let mut allocator = UefiPageAllocator::new();
        assert!(allocator.allocate_pages(4096, 0).is_err());
        assert!(allocator.allocate_pages(4096, 3).is_err());
    }

    #[test]
    fn uefi_page_allocator_allocates_and_copies_on_host() {
        let mut allocator = UefiPageAllocator::new();
        let host_phys = allocator.allocate_pages(4096, 4096).expect("allocate");
        assert_eq!(host_phys & 0xFFF, 0);
        assert!(allocator.copy_to_pages(host_phys, &[0xAB; 16]).is_ok());
    }

    #[test]
    fn uefi_page_allocator_copy_rejects_unknown_target() {
        let mut allocator = UefiPageAllocator::new();
        assert!(allocator.copy_to_pages(0x1000, &[0u8; 16]).is_err());
    }

    #[test]
    fn pages_for_bytes_rounds_up_to_page_count() {
        assert_eq!(pages_for_bytes(1).expect("one byte"), 1);
        assert_eq!(pages_for_bytes(4096).expect("one page"), 1);
        assert_eq!(pages_for_bytes(4097).expect("page plus one"), 2);
    }

    #[test]
    fn pages_for_bytes_rejects_overflow() {
        assert!(pages_for_bytes(usize::MAX).is_err());
    }

    #[cfg(test)]
    impl UefiPageAllocator {
        fn test_with_page(host_phys: u64, capacity: usize) -> Self {
            let mut bytes = alloc::vec![0u8; capacity];
            let ptr = bytes.as_mut_ptr();
            core::mem::forget(bytes);
            Self {
                pages: alloc::vec![(host_phys, ptr, capacity)],
            }
        }
    }

    #[test]
    fn uefi_page_allocator_copy_rejects_over_capacity() {
        let mut allocator = UefiPageAllocator::test_with_page(0x1000, 16);
        assert!(allocator.copy_to_pages(0x1000, &[0u8; 32]).is_err());
    }

    #[test]
    fn uefi_page_allocator_copy_accepts_within_capacity() {
        let mut allocator = UefiPageAllocator::test_with_page(0x1000, 16);
        assert!(allocator.copy_to_pages(0x1000, &[0u8; 8]).is_ok());
    }
}
