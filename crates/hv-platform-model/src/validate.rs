//! Fail-closed comparison between compile-time requirements and observed platform.

use hv_config_model::{FeatureRequirement, PlatformRequirements, SmtPolicy};

use crate::error::{PlatformError, PlatformErrorKind, PlatformWarning};
use crate::observed::ObservedPlatform;
use crate::validated::ValidatedPlatform;

/// Validates an observed platform against compile-time requirements.
pub fn validate_platform(
    requirements: &PlatformRequirements,
    observed: &ObservedPlatform,
) -> Result<(ValidatedPlatform, Vec<PlatformWarning>), PlatformError> {
    let mut warnings = Vec::new();

    if observed.arch_requirement()? != requirements.arch {
        return Err(PlatformError::new(
            PlatformErrorKind::Validation,
            format!(
                "observed arch '{}' does not match required {:?}",
                observed.arch, requirements.arch
            ),
        ));
    }

    check_feature("vmx", requirements.vmx, observed.vmx, &mut warnings)?;
    check_feature("ept", requirements.ept, observed.ept, &mut warnings)?;
    check_feature("vtd", requirements.vtd, observed.vtd, &mut warnings)?;
    check_feature(
        "interrupt_remapping",
        requirements.interrupt_remapping,
        observed.interrupt_remapping,
        &mut warnings,
    )?;
    check_feature(
        "x2apic",
        requirements.x2apic,
        observed.x2apic,
        &mut warnings,
    )?;
    check_feature(
        "invariant_tsc",
        requirements.invariant_tsc,
        observed.invariant_tsc,
        &mut warnings,
    )?;
    check_feature("vpid", requirements.vpid, observed.vpid, &mut warnings)?;
    check_feature(
        "vmx_preemption_timer",
        requirements.vmx_preemption_timer,
        observed.vmx_preemption_timer,
        &mut warnings,
    )?;
    check_feature("nx", requirements.nx, observed.nx, &mut warnings)?;

    if observed.physical_cores < requirements.min_physical_cores {
        return Err(PlatformError::new(
            PlatformErrorKind::Validation,
            format!(
                "observed physical cores {} below required {}",
                observed.physical_cores, requirements.min_physical_cores
            ),
        ));
    }

    if observed.ram_bytes.bytes() < requirements.min_ram_bytes.bytes() {
        return Err(PlatformError::new(
            PlatformErrorKind::Validation,
            format!(
                "observed RAM {} bytes below required {} bytes",
                observed.ram_bytes.bytes(),
                requirements.min_ram_bytes.bytes()
            ),
        ));
    }

    validate_page_sizes(requirements, observed)?;
    validate_smt_policy(requirements, observed, &mut warnings)?;
    validate_pci_devices(requirements, observed)?;

    Ok((ValidatedPlatform::new(observed.clone()), warnings))
}

fn check_feature(
    name: &str,
    requirement: FeatureRequirement,
    observed: bool,
    warnings: &mut Vec<PlatformWarning>,
) -> Result<(), PlatformError> {
    match requirement {
        FeatureRequirement::Required if !observed => Err(PlatformError::new(
            PlatformErrorKind::Validation,
            format!("required feature '{name}' is absent"),
        )),
        FeatureRequirement::Disabled if observed => Err(PlatformError::new(
            PlatformErrorKind::Validation,
            format!("feature '{name}' must be disabled but is present"),
        )),
        FeatureRequirement::Preferred if !observed => {
            warnings.push(PlatformWarning::new(format!(
                "preferred feature '{name}' is absent"
            )));
            Ok(())
        }
        FeatureRequirement::Required
        | FeatureRequirement::Disabled
        | FeatureRequirement::Optional
        | FeatureRequirement::Preferred => Ok(()),
    }
}

fn validate_page_sizes(
    requirements: &PlatformRequirements,
    observed: &ObservedPlatform,
) -> Result<(), PlatformError> {
    for required in &requirements.page_sizes.sizes {
        if !observed.page_sizes.contains(required) {
            return Err(PlatformError::new(
                PlatformErrorKind::Validation,
                format!("required page size {required} bytes is not supported"),
            ));
        }
    }
    Ok(())
}

fn validate_smt_policy(
    requirements: &PlatformRequirements,
    observed: &ObservedPlatform,
    warnings: &mut Vec<PlatformWarning>,
) -> Result<(), PlatformError> {
    match requirements.smt_policy {
        SmtPolicy::Disabled if observed.smt_enabled => Err(PlatformError::new(
            PlatformErrorKind::Validation,
            "SMT must be disabled but is enabled",
        )),
        SmtPolicy::ExclusiveCore if observed.smt_enabled => {
            warnings.push(PlatformWarning::new(
                "SMT is enabled while exclusive core policy is configured",
            ));
            Ok(())
        }
        SmtPolicy::Disabled
        | SmtPolicy::ExclusiveCore
        | SmtPolicy::SamePartitionSiblings
        | SmtPolicy::AllowCrossPartition => Ok(()),
    }
}

fn validate_pci_devices(
    requirements: &PlatformRequirements,
    observed: &ObservedPlatform,
) -> Result<(), PlatformError> {
    for expected in &requirements.expected_pci_devices {
        if !observed.pci_devices.iter().any(|bdf| *bdf == expected.bdf) {
            return Err(PlatformError::new(
                PlatformErrorKind::Validation,
                format!(
                    "expected PCI device {:04x}:{:02x}:{:02x}.{} is not present",
                    expected.bdf.segment.raw(),
                    expected.bdf.bus.raw(),
                    expected.bdf.device.raw(),
                    expected.bdf.function.raw()
                ),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::observed::parse_observed_platform_json;
    use hv_config_model::compile_config_from_str;

    #[test]
    fn reference_observed_platform_passes_reference_requirements() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let json = include_str!("../tests/fixtures/observed/qemu_reference.json");
        let observed = parse_observed_platform_json(json).expect("parse");
        let (validated, warnings) =
            validate_platform(&compiled.requirements, &observed).expect("validate");
        assert_eq!(validated.observed.physical_cores, 4);
        assert!(warnings.is_empty());
    }

    #[test]
    fn missing_required_vmx_is_rejected() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let json = include_str!("../tests/fixtures/observed/missing_vmx.json");
        let observed = parse_observed_platform_json(json).expect("parse");
        let err = validate_platform(&compiled.requirements, &observed).expect_err("must fail");
        assert_eq!(err.kind, PlatformErrorKind::Validation);
        assert!(err.message.contains("vmx"));
    }

    #[test]
    fn insufficient_ram_is_rejected() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let json = include_str!("../tests/fixtures/observed/qemu_reference.json");
        let mut observed = parse_observed_platform_json(json).expect("parse");
        observed.ram_bytes = hv_types::ByteSize::new(1);
        let err = validate_platform(&compiled.requirements, &observed).expect_err("must fail");
        assert!(err.message.contains("RAM"));
    }
}
