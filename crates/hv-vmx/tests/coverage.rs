//! Additional coverage for VMX error paths.

use hv_config_model::compile_config_from_str;
use hv_platform_model::plan_static_platform_ir;
use hv_types::{ByteSize, HostPhysAddr};
use hv_vmx::{plan_vmx_init, VmxErrorKind};

#[test]
fn plan_vmx_init_rejects_undersized_reserve() {
    let reserve = hv_platform_model::PlannedHypervisorReserve {
        host_phys: HostPhysAddr::new(0x1000),
        size: ByteSize::new(4095),
    };
    let err = plan_vmx_init(&reserve).expect_err("must fail");
    assert_eq!(err.kind, VmxErrorKind::Planning);
}

#[test]
fn plan_vmx_init_rejects_unaligned_reserve_base() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let mut layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    layout.hypervisor_reserve.host_phys = HostPhysAddr::new(0x1001);
    let err = plan_vmx_init(&layout.hypervisor_reserve).expect_err("must fail");
    assert_eq!(err.kind, VmxErrorKind::Planning);
}

#[test]
fn plan_vmx_init_rejects_unaligned_reserve_size() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let mut layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    layout.hypervisor_reserve.size = ByteSize::new(4097);
    let err = plan_vmx_init(&layout.hypervisor_reserve).expect_err("must fail");
    assert_eq!(err.kind, VmxErrorKind::Planning);
}

#[test]
fn vmx_error_display_includes_kind_and_message() {
    use hv_vmx::{VmxError, VmxErrorKind};
    let err = VmxError::new(VmxErrorKind::Backend, "mock failure");
    assert!(format!("{err}").contains("vmx backend error"));
    assert!(format!("{err}").contains("mock failure"));
    assert!(format!("{}", VmxErrorKind::Planning).contains("planning"));
    assert!(format!("{}", VmxErrorKind::Requirements).contains("requirements"));
}

#[test]
fn vmx_error_converts_from_platform_error() {
    use hv_platform_model::{PlatformError, PlatformErrorKind};
    use hv_vmx::{VmxError, VmxErrorKind};
    let platform = PlatformError::new(PlatformErrorKind::Planning, "layout mismatch");
    let err = VmxError::from(platform);
    assert_eq!(err.kind, VmxErrorKind::Planning);
    assert_eq!(err.message, "layout mismatch");
}
