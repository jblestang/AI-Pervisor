//! Additional coverage for VT-d hardware programming paths.

use hv_config_model::compile_config_from_str;
use hv_platform_model::plan_static_platform_ir;
use hv_types::{PciBdf, PciBus, PciDevice, PciFunction, PciSegment};
use hv_vtd::{encode_vtd_context_entry, plan_vtd_init, program_vtd_tables};

#[test]
fn encode_vtd_context_entry_without_interrupt_remapping_flag() {
    let assignment = hv_vtd::VtdDeviceAssignment {
        bdf: PciBdf::new(
            PciSegment::new(0),
            PciBus::new(0),
            PciDevice::new(3),
            PciFunction::new(0),
        ),
        vm_id: 1,
    };
    let encoded = encode_vtd_context_entry(&assignment, false);
    assert!(!encoded.interrupt_remapping);
}

#[test]
fn encode_vtd_context_entry_sets_interrupt_remapping_flag() {
    let assignment = hv_vtd::VtdDeviceAssignment {
        bdf: PciBdf::new(
            PciSegment::new(0),
            PciBus::new(0),
            PciDevice::new(3),
            PciFunction::new(0),
        ),
        vm_id: 2,
    };
    let encoded = encode_vtd_context_entry(&assignment, true);
    assert!(encoded.interrupt_remapping);
    assert_ne!(encoded.context_flags & (1 << 1), 0);
}

#[test]
fn program_vtd_tables_accepts_empty_pci_topology() {
    let yaml = include_str!("../../../configs/ovmf-smoke.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    let plan = plan_vtd_init(&layout, false).expect("vtd");
    let tables = program_vtd_tables(&plan).expect("program");
    assert!(tables.assignments.is_empty());
}

#[test]
fn program_vtd_tables_accepts_reference_layout() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    let plan = plan_vtd_init(&layout, true).expect("vtd");
    let tables = program_vtd_tables(&plan).expect("program");
    assert_eq!(tables.assignments.len(), 2);
}
