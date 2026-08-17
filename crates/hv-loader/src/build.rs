//! Boot info blob construction for loader integration.

use core::mem::size_of;

use alloc::vec::Vec;

use hv_boot_abi::{BootError, BootErrorKind, BootInfoDescriptor, BOOT_ABI_VERSION, BOOT_INFO_MAGIC};
use hv_types::SHA256_DIGEST_BYTES;

/// One payload section stored inside a boot info blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootInfoSection {
    /// Descriptor kind identifier.
    pub kind: u32,
    /// Section payload bytes.
    pub data: Vec<u8>,
}

/// Builds a boot info blob from ordered sections.
pub fn build_boot_info_blob(
    config_digest: [u8; SHA256_DIGEST_BYTES],
    sections: &[BootInfoSection],
) -> Result<Vec<u8>, BootError> {
    let header_size = size_of::<hv_boot_abi::BootInfoHeader>();
    let descriptor_table_size = sections
        .len()
        .checked_mul(size_of::<BootInfoDescriptor>())
        .ok_or(BootError::new(
            BootErrorKind::Bounds,
            "descriptor table overflow",
        ))?;
    let payload_size = sections.iter().try_fold(0usize, |acc, section| {
        acc.checked_add(section.data.len()).ok_or(BootError::new(
            BootErrorKind::Bounds,
            "payload size overflow",
        ))
    })?;
    let total_size = header_size
        .checked_add(descriptor_table_size)
        .and_then(|size| size.checked_add(payload_size))
        .ok_or(BootError::new(
            BootErrorKind::Bounds,
            "boot info total size overflow",
        ))?;

    let mut blob = Vec::new();
    append_header(
        &mut blob,
        config_digest,
        total_size as u32,
        sections.len() as u32,
    )?;

    let mut payload_offset = (header_size + descriptor_table_size) as u32;
    for section in sections {
        append_descriptor(
            &mut blob,
            &BootInfoDescriptor {
                kind: section.kind,
                offset: payload_offset,
                size: section.data.len() as u32,
            },
        )?;
        payload_offset = payload_offset
            .checked_add(section.data.len() as u32)
            .ok_or(BootError::new(
                BootErrorKind::Bounds,
                "payload offset overflow",
            ))?;
    }
    for section in sections {
        blob.extend_from_slice(&section.data);
    }
    Ok(blob)
}

fn append_header(
    blob: &mut Vec<u8>,
    config_digest: [u8; SHA256_DIGEST_BYTES],
    total_size: u32,
    descriptor_count: u32,
) -> Result<(), BootError> {
    blob.extend_from_slice(&BOOT_INFO_MAGIC);
    blob.extend_from_slice(&BOOT_ABI_VERSION.to_le_bytes());
    blob.extend_from_slice(&total_size.to_le_bytes());
    blob.extend_from_slice(&config_digest);
    blob.extend_from_slice(&(size_of::<hv_boot_abi::BootInfoHeader>() as u32).to_le_bytes());
    blob.extend_from_slice(&descriptor_count.to_le_bytes());
    Ok(())
}

fn append_descriptor(blob: &mut Vec<u8>, descriptor: &BootInfoDescriptor) -> Result<(), BootError> {
    blob.extend_from_slice(&descriptor.kind.to_le_bytes());
    blob.extend_from_slice(&descriptor.offset.to_le_bytes());
    blob.extend_from_slice(&descriptor.size.to_le_bytes());
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_boot_abi::{descriptor_kind, validate_rsdp_section, AcpiRsdp, BootInfoView};

    #[test]
    fn build_boot_info_blob_is_parseable() {
        let digest = [0xAA; SHA256_DIGEST_BYTES];
        let blob = build_boot_info_blob(
            digest,
            &[
                BootInfoSection {
                    kind: descriptor_kind::MEMORY_MAP,
                    data: vec![0u8; 16],
                },
                BootInfoSection {
                    kind: descriptor_kind::RSDP,
                    data: AcpiRsdp::encode_reference_v2().to_vec(),
                },
            ],
        )
        .expect("build");
        let view = BootInfoView::parse(&blob).expect("parse");
        view.verify_config_digest(&digest).expect("digest");
        validate_rsdp_section(view.rsdp_section().expect("rsdp").expect("present"))
            .expect("rsdp");
    }
}
