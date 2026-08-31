//! Integration tests for live instruction modules.

#![allow(clippy::expect_used)]

use hv_x86_cpu::{
    execute_vmxon, execute_vtd_enable, last_vtd_enable_intent, live_execution_environment_ready,
    live_execution_runtime_enabled, read_vmx_basic_msr, run_vmxon_cpu_seam,
    vmx_revision_from_basic_msr, CpuInstructionDisposition, CpuSeamErrorKind, VtdEnableIntent,
    HV_X86_LIVE_INSTRUCTIONS_ENABLED, HV_X86_LIVE_INSTRUCTIONS_ENV,
};

#[test]
fn live_execution_runtime_stays_disabled_without_env_var() {
    if cfg!(feature = "firmware-live-execution")
        || std::env::var(HV_X86_LIVE_INSTRUCTIONS_ENV).ok().as_deref()
            == Some(HV_X86_LIVE_INSTRUCTIONS_ENABLED)
    {
        assert!(live_execution_runtime_enabled());
    } else {
        assert!(!live_execution_runtime_enabled());
    }
    assert!(!live_execution_environment_ready());
}

#[test]
fn read_vmx_basic_msr_rejects_userspace_live_path() {
    assert!(read_vmx_basic_msr().is_err());
}

#[test]
fn vmx_revision_from_basic_msr_extracts_low_bits() {
    assert_eq!(vmx_revision_from_basic_msr(0x0000_00AB_0000_0000), 0);
}

#[test]
fn execute_vtd_enable_records_intent_only_when_environment_ready() {
    assert!(execute_vtd_enable(true).is_err());
    assert_eq!(
        last_vtd_enable_intent(),
        VtdEnableIntent {
            recorded: false,
            interrupt_remapping: false,
        }
    );
}

#[test]
fn run_vmxon_cpu_seam_keeps_seam_validated_without_live_environment() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = hv_config_model::compile_config_from_str(yaml).expect("compile");
    let layout = hv_platform_model::plan_static_platform_ir(&compiled.intent).expect("plan");
    let plan = hv_vmx::plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
    let region =
        hv_vmx::program_vmxon_region(&plan, hv_vmx::REFERENCE_VMXON_REVISION).expect("program");
    let outcome = run_vmxon_cpu_seam(&region).expect("seam");
    assert_ne!(outcome.disposition, CpuInstructionDisposition::Executed);
}

#[test]
fn invalid_vmxon_operand_fails_before_live_environment_gate() {
    let err = execute_vmxon(0x1001).expect_err("must fail validation before environment gate");
    assert_eq!(err.kind, CpuSeamErrorKind::InvalidInput);
}
