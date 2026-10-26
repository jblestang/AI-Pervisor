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
    assert!(matches!(
        c.datapath.backend,
        demo_config::VfBackend::Sriov | demo_config::VfBackend::Inject
    ));
}
