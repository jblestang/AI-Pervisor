//! Host-side tests for NicBackend: real Inject queue + Sriov fail-fast.
//! Inject is an in-process frame queue (production degraded path), not a mock of Sriov.

use pci_vf::{
    open_from_config, Backend, BackendKind, FlattenedNicConfig, InjectNic, NicBackend, NicError,
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
}
