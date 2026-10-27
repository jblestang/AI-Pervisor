#[test]
fn rejects_incomplete_config() {
    let toml = r#"
        [guests]
        wan_memory_mb = 64
        # lan_memory_mb missing
    "#;
    let err = demo_config::DemoConfig::parse_str(toml).unwrap_err();
    assert!(err.to_string().contains("lan_memory_mb"));
}

#[test]
fn loads_example_config() {
    let s = include_str!("../../../config/demo.example.toml");
    let c = demo_config::DemoConfig::parse_str(s).expect("example must be complete");
    assert_eq!(c.bench.frame_size, 64);
    assert_eq!(c.bench.report_path, "target/bench-report.json");
    assert!(matches!(
        c.datapath.backend,
        demo_config::VfBackend::Sriov | demo_config::VfBackend::Inject
    ));
}

#[test]
fn sriov_requires_lan_bdf_and_rejects_bad_ring_pow2() {
    let mut s = include_str!("../../../config/demo.example.toml").to_string();
    s = s.replace("lan_vf_bdf = \"0000:01:10.1\"", "lan_vf_bdf = \"\"");
    let err = demo_config::DemoConfig::parse_str(&s).unwrap_err();
    assert!(err.to_string().contains("lan_vf_bdf"));

    let mut s = include_str!("../../../config/demo.example.toml").to_string();
    s = s.replace("capacity_pow2 = 1024", "capacity_pow2 = 1000");
    let err = demo_config::DemoConfig::parse_str(&s).unwrap_err();
    assert!(err.to_string().contains("capacity_pow2") || err.to_string().contains("power"));
}

#[test]
fn env_override_rejects_unknown_backend() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../config/demo.example.toml"
    );
    std::env::set_var("HV_DEMO_CONFIG", path);
    std::env::set_var("HV_VF_BACKEND", "not-a-backend");
    let err = demo_config::DemoConfig::load("/nonexistent.toml").unwrap_err();
    std::env::remove_var("HV_DEMO_CONFIG");
    std::env::remove_var("HV_VF_BACKEND");
    assert!(err.to_string().contains("HV_VF_BACKEND"));
}
