//! Host-side tests for NicBackend: real Inject queue + Sriov fail-fast.
//! Inject is an in-process frame queue (production degraded path), not a mock of Sriov.

use pci_vf::{
    backend_kind_from_str, open_from_config, Backend, BackendKind, FlattenedNicConfig, InjectNic,
    NicBackend, NicError, SriovNic,
};

#[test]
fn inject_send_recv_roundtrip() {
    let mut nic = InjectNic::with_capacity(8);
    assert!(!nic.rx_ready());

    nic.inject_rx(b"hello-64b-frame................................").unwrap();
    assert!(nic.rx_ready());

    let mut buf = [0u8; 64];
    let n = nic.try_recv(&mut buf).unwrap();
    assert_eq!(&buf[..n], b"hello-64b-frame................................");
    assert!(!nic.rx_ready());

    nic.try_send(b"reply-from-guest.................................").unwrap();
    let tx = nic.take_tx().expect("tx frame");
    assert_eq!(&tx[..], b"reply-from-guest.................................");
}

#[test]
fn inject_open_from_config_selects_inject_backend() {
    let cfg = FlattenedNicConfig {
        kind: BackendKind::Inject,
        allow_inject_fallback: false,
        vf_bdf: None,
        inject_queue_capacity: 4,
    };
    let mut backend = open_from_config(&cfg).expect("inject open");
    assert!(matches!(backend, Backend::Inject(_)));

    if let Backend::Inject(ref mut nic) = backend {
        nic.inject_rx(&[1, 2, 3, 4]).unwrap();
        let mut buf = [0u8; 8];
        assert_eq!(nic.try_recv(&mut buf).unwrap(), 4);
        assert_eq!(&buf[..4], &[1, 2, 3, 4]);
    }
}

#[test]
fn sriov_required_missing_bdf_fails_fast() {
    let cfg = FlattenedNicConfig {
        kind: BackendKind::Sriov,
        allow_inject_fallback: false,
        vf_bdf: None,
        inject_queue_capacity: 4,
    };
    let err = open_from_config(&cfg).expect_err("must not silent-mock");
    assert!(
        matches!(err, NicError::MissingBdf),
        "expected MissingBdf, got {err:?}"
    );
}

#[test]
fn sriov_required_empty_bdf_fails_fast() {
    let cfg = FlattenedNicConfig {
        kind: BackendKind::Sriov,
        allow_inject_fallback: false,
        vf_bdf: Some(""),
        inject_queue_capacity: 4,
    };
    let err = open_from_config(&cfg).expect_err("empty BDF is missing");
    assert!(matches!(err, NicError::MissingBdf));
}

#[test]
fn sriov_missing_device_errors_without_fallback() {
    // BDF present in config but no VF hardware in CI → DeviceMissing, not Inject.
    let cfg = FlattenedNicConfig {
        kind: BackendKind::Sriov,
        allow_inject_fallback: false,
        vf_bdf: Some("0000:01:10.0"),
        inject_queue_capacity: 4,
    };
    let err = open_from_config(&cfg).expect_err("no silent substitute");
    assert!(
        matches!(err, NicError::DeviceMissing { .. }),
        "expected DeviceMissing, got {err:?}"
    );
}

#[test]
fn sriov_missing_device_falls_back_to_inject_when_allowed() {
    let cfg = FlattenedNicConfig {
        kind: BackendKind::Sriov,
        allow_inject_fallback: true,
        vf_bdf: Some("0000:01:10.0"),
        inject_queue_capacity: 2,
    };
    let backend = open_from_config(&cfg).expect("fallback inject");
    assert!(matches!(backend, Backend::Inject(_)));
}

#[test]
fn flattened_config_from_demo_config_fields() {
    // Callers map DemoConfig → FlattenedNicConfig; pci-vf never hardcodes BDFs.
    let demo = demo_config::DemoConfig::parse_str(include_str!("../../config/demo.example.toml"))
        .expect("example config");
    let kind = match demo.datapath.backend {
        demo_config::VfBackend::Sriov => BackendKind::Sriov,
        demo_config::VfBackend::Inject => BackendKind::Inject,
    };
    let wan = FlattenedNicConfig {
        kind,
        allow_inject_fallback: demo.datapath.allow_inject_fallback,
        vf_bdf: demo.datapath.wan_vf_bdf.as_deref(),
        inject_queue_capacity: demo.ring.frame_capacity as usize,
    };
    assert_eq!(wan.kind, BackendKind::Sriov);
    assert_eq!(wan.vf_bdf, Some("0000:01:10.0"));
    // Example has allow_inject_fallback=false and no VF in CI → DeviceMissing.
    let err = open_from_config(&wan).expect_err("lab BDF not present in CI");
    assert!(matches!(err, NicError::DeviceMissing { .. }));
}

#[test]
fn inject_queue_full_and_empty_errors() {
    let mut nic = InjectNic::with_capacity(1);
    assert_eq!(nic.capacity(), 1);
    nic.inject_rx(&[0u8; 16]).unwrap();
    assert!(matches!(
        nic.inject_rx(&[1u8; 16]),
        Err(NicError::QueueFull)
    ));

    let mut buf = [0u8; 8];
    assert!(matches!(nic.try_recv(&mut buf), Err(NicError::BufferTooSmall)));

    let mut big = [0u8; 32];
    assert_eq!(nic.try_recv(&mut big).unwrap(), 16);
    assert!(matches!(nic.try_recv(&mut big), Err(NicError::QueueEmpty)));
    assert!(!nic.rx_ready());

    nic.try_send(&[9u8; 4]).unwrap();
    assert!(matches!(nic.try_send(&[8u8; 4]), Err(NicError::QueueFull)));
}

#[test]
fn backend_enum_dispatch_inject_and_sriov_stub() {
    let cfg = FlattenedNicConfig {
        kind: BackendKind::Inject,
        allow_inject_fallback: false,
        vf_bdf: None,
        inject_queue_capacity: 4,
    };
    let mut backend = open_from_config(&cfg).expect("inject");
    assert!(!backend.rx_ready());
    if let Backend::Inject(ref mut nic) = backend {
        nic.inject_rx(b"abc").unwrap();
    }
    assert!(backend.rx_ready());
    let mut buf = [0u8; 8];
    assert_eq!(backend.try_recv(&mut buf).unwrap(), 3);
    backend.try_send(b"xy").unwrap();

    // Pre-MMIO Sriov stub via Backend enum (no lab VF required).
    let mut sriov = Backend::Sriov(SriovNic::stub_pre_mmio("0000:00:00.0"));
    assert!(!sriov.rx_ready());
    assert!(matches!(sriov.try_recv(&mut buf), Err(NicError::QueueEmpty)));
    assert!(matches!(sriov.try_send(b"z"), Err(NicError::IoFailure)));
}

#[test]
fn backend_kind_from_str_parses_config_names() {
    assert_eq!(backend_kind_from_str("sriov"), Some(BackendKind::Sriov));
    assert_eq!(backend_kind_from_str(" Inject "), Some(BackendKind::Inject));
    assert_eq!(backend_kind_from_str("nope"), None);
}

#[test]
fn nic_error_display_covers_variants() {
    let msgs = [
        NicError::MissingBdf.to_string(),
        NicError::DeviceMissing {
            bdf: "0000:01:10.0".into(),
        }
        .to_string(),
        NicError::BufferTooSmall.to_string(),
        NicError::QueueFull.to_string(),
        NicError::QueueEmpty.to_string(),
        NicError::IoFailure.to_string(),
    ];
    assert!(msgs[0].contains("BDF") || msgs[0].contains("bdf") || msgs[0].contains("missing"));
    assert!(msgs[1].contains("0000:01:10.0"));
    assert!(msgs.iter().all(|m| !m.is_empty()));
}

#[test]
fn sriov_open_succeeds_against_fake_sysfs_tree() {
    let tmp = std::env::temp_dir().join(format!(
        "hv-vf-sysfs-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let bdf = "0000:0a:00.1";
    let dev = tmp.join(bdf);
    std::fs::create_dir_all(&dev).expect("fake sysfs node");
    // Task 10: open requires a vfio-pci driver symlink (bind checklist).
    let drivers = tmp.join("drivers/vfio-pci");
    std::fs::create_dir_all(&drivers).expect("fake vfio-pci driver dir");
    std::os::unix::fs::symlink(&drivers, dev.join("driver")).expect("driver symlink");
    // SAFETY: test-only env for probe root; cleaned up below.
    std::env::set_var("HV_TEST_VF_SYSFS_ROOT", &tmp);
    let cfg = FlattenedNicConfig {
        kind: BackendKind::Sriov,
        allow_inject_fallback: false,
        vf_bdf: Some(bdf),
        inject_queue_capacity: 2,
    };
    let backend = open_from_config(&cfg);
    std::env::remove_var("HV_TEST_VF_SYSFS_ROOT");
    let _ = std::fs::remove_dir_all(&tmp);
    let mut backend = backend.expect("Sriov open via fake sysfs");
    assert!(matches!(backend, Backend::Sriov(_)));
    assert!(!backend.rx_ready());
    let mut buf = [0u8; 4];
    assert!(matches!(backend.try_recv(&mut buf), Err(NicError::QueueEmpty)));
}

#[test]
fn sriov_open_rejects_unbound_vf_sysfs_node() {
    let tmp = std::env::temp_dir().join(format!(
        "hv-vf-unbound-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let bdf = "0000:0b:00.1";
    std::fs::create_dir_all(tmp.join(bdf)).expect("fake unbound VF");
    std::env::set_var("HV_TEST_VF_SYSFS_ROOT", &tmp);
    let err = SriovNic::open(bdf).expect_err("unbound VF must not open");
    std::env::remove_var("HV_TEST_VF_SYSFS_ROOT");
    let _ = std::fs::remove_dir_all(&tmp);
    assert!(matches!(err, NicError::DeviceMissing { .. }));
}
