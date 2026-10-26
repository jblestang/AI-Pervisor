//! QEMU argv builder — values come **only** from [`demo_config::DemoConfig`].

use std::path::{Path, PathBuf};

use demo_config::{DemoConfig, VfBackend};

/// How the nested VM is launched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NestedMode {
    /// Interactive / long-running (`run-nested`).
    Run,
    /// Smoke: same machine, serial captured by the harness.
    Smoke,
}

/// Resolved image paths (workspace-relative config paths → absolute).
#[derive(Clone, Debug)]
pub struct NestedImages {
    pub hv: PathBuf,
    pub guest_wan: PathBuf,
    pub guest_lan: PathBuf,
    pub flat_cfg: PathBuf,
}

/// Preflight / launch errors with actionable messages.
#[derive(Debug)]
pub enum NestedError {
    Message(String),
}

impl std::fmt::Display for NestedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NestedError::Message(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for NestedError {}

/// Objcopy freestanding `hv` ELF → Multiboot ADDR raw binary (QEMU rejects ELF64).
pub fn ensure_hv_multiboot_bin(hv_elf: &Path) -> Result<PathBuf, NestedError> {
    let bin = hv_elf.with_extension("bin");
    let status = std::process::Command::new("objcopy")
        .args(["-O", "binary"])
        .arg(hv_elf)
        .arg(&bin)
        .status()
        .map_err(|e| {
            NestedError::Message(format!(
                "objcopy failed ({e}); install binutils for Multiboot hv.bin"
            ))
        })?;
    if !status.success() {
        return Err(NestedError::Message(format!(
            "objcopy -O binary {} → {} failed",
            hv_elf.display(),
            bin.display()
        )));
    }
    Ok(bin)
}

/// Resolve a config path against `root`, preferring the exact path then
/// `release`↔`debug` swap so example release paths work after `xtask build`.
pub fn resolve_image(root: &Path, configured: &str) -> Result<PathBuf, NestedError> {
    let p = PathBuf::from(configured);
    let abs = if p.is_absolute() {
        p
    } else {
        root.join(p)
    };
    if abs.is_file() {
        return Ok(abs);
    }
    let alt = if configured.contains("/release/") {
        root.join(configured.replace("/release/", "/debug/"))
    } else if configured.contains("/debug/") {
        root.join(configured.replace("/debug/", "/release/"))
    } else {
        abs.clone()
    };
    if alt.is_file() {
        return Ok(alt);
    }
    Err(NestedError::Message(format!(
        "image not found: {} (also tried {})\n  hint: run `cargo xtask build` first",
        abs.display(),
        alt.display()
    )))
}

/// Check `/dev/kvm` and nested VT-x parameter.
///
/// When Multiboot runs under TCG (default), missing KVM is a **warning** — L1 inject
/// smoke can still pass. Forcing `HV_QEMU_ACCEL=kvm` makes these checks hard errors.
pub fn check_kvm_nested() -> Result<(), NestedError> {
    let want_kvm = matches!(
        std::env::var("HV_QEMU_ACCEL")
            .unwrap_or_else(|_| "tcg".into())
            .to_ascii_lowercase()
            .as_str(),
        "kvm" | "kvm:tcg" | "auto"
    );
    let soft = |msg: String| -> Result<(), NestedError> {
        if want_kvm {
            Err(NestedError::Message(msg))
        } else {
            eprintln!("xtask: warning: {msg}");
            Ok(())
        }
    };

    let kvm = Path::new("/dev/kvm");
    if !kvm.exists() {
        return soft(
            "/dev/kvm absent (nested VT-x unavailable).\n  \
             hint: host unit tests still work; Multiboot smoke defaults to TCG"
                .into(),
        );
    }
    if let Err(e) = std::fs::File::open(kvm) {
        return soft(format!(
            "cannot open /dev/kvm: {e}\n  hint: add user to the kvm group or chmod the device"
        ));
    }
    let nested = Path::new("/sys/module/kvm_intel/parameters/nested");
    if nested.exists() {
        let v = std::fs::read_to_string(nested).unwrap_or_default();
        let v = v.trim();
        if v != "Y" && v != "1" {
            return soft(format!(
                "kvm_intel nested={v:?} (need Y/1).\n  \
                 hint: modprobe kvm_intel nested=1  (see docs/HARNESS.md)"
            ));
        }
    }
    Ok(())
}

/// Sysfs root for PCI devices (`HV_TEST_VF_SYSFS_ROOT` overrides for host tests).
fn pci_sysfs_root() -> PathBuf {
    std::env::var_os("HV_TEST_VF_SYSFS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/sys/bus/pci/devices"))
}

/// Sysfs presence check for a PCI BDF (`0000:bb:dd.f`).
pub fn vf_sysfs_present(bdf: &str) -> bool {
    let bdf = bdf.trim();
    if bdf.is_empty() {
        return false;
    }
    pci_sysfs_root().join(bdf).is_dir()
}

/// True when the VF sysfs `driver` symlink leaf is `vfio-pci`.
pub fn vf_vfio_bound(bdf: &str) -> bool {
    let bdf = bdf.trim();
    if bdf.is_empty() {
        return false;
    }
    let link = pci_sysfs_root().join(bdf).join("driver");
    match std::fs::read_link(&link) {
        Ok(target) => target
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s == "vfio-pci")
            .unwrap_or(false),
        Err(_) => false,
    }
}

/// Describe why a configured VF is not ready for Sriov passthrough.
fn vf_readiness(bdf: &str) -> Result<(), String> {
    let bdf = bdf.trim();
    if bdf.is_empty() {
        return Err("BDF empty".into());
    }
    if !vf_sysfs_present(bdf) {
        return Err(format!(
            "sysfs node missing (/sys/bus/pci/devices/{bdf} or HV_TEST_VF_SYSFS_ROOT)"
        ));
    }
    if !vf_vfio_bound(bdf) {
        let driver = std::fs::read_link(pci_sysfs_root().join(bdf).join("driver"))
            .ok()
            .and_then(|p| {
                p.file_name()
                    .map(|s| s.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "unbound".into());
        return Err(format!(
            "not bound to vfio-pci (current driver={driver}); see docs/HARNESS.md § VF bind checklist"
        ));
    }
    Ok(())
}

/// Apply Inject fallback policy when Sriov VFs are missing / unbound.
///
/// Returns the backend that will actually be used (may rewrite `cfg` in place).
/// Never silently substitutes Inject when `allow_inject_fallback=false`.
pub fn resolve_datapath_for_launch(cfg: &mut DemoConfig) -> Result<VfBackend, NestedError> {
    match cfg.datapath.backend {
        VfBackend::Inject => Ok(VfBackend::Inject),
        VfBackend::Sriov => {
            let wan = cfg.datapath.wan_vf_bdf.as_deref().unwrap_or("").trim();
            let lan = cfg.datapath.lan_vf_bdf.as_deref().unwrap_or("").trim();
            let wan_ready = vf_readiness(wan);
            let lan_ready = vf_readiness(lan);
            if wan_ready.is_ok() && lan_ready.is_ok() {
                return Ok(VfBackend::Sriov);
            }
            let wan_detail = wan_ready.err().unwrap_or_else(|| "ok".into());
            let lan_detail = lan_ready.err().unwrap_or_else(|| "ok".into());
            if cfg.datapath.allow_inject_fallback {
                eprintln!(
                    "xtask: Sriov VF not ready \
                     (wan={wan:?}: {wan_detail}; lan={lan:?}: {lan_detail}); \
                     allow_inject_fallback=true → using Inject"
                );
                cfg.datapath.backend = VfBackend::Inject;
                Ok(VfBackend::Inject)
            } else {
                Err(NestedError::Message(format!(
                    "datapath.backend=sriov but VF not ready for passthrough \
                     (wan={wan:?}: {wan_detail}; lan={lan:?}: {lan_detail}) \
                     and allow_inject_fallback=false.\n  \
                     hint: complete the VF bind checklist in docs/HARNESS.md \
                     (IOMMU + vfio-pci + nested), or set datapath.backend=inject \
                     / allow_inject_fallback=true, or HV_VF_BACKEND=inject"
                )))
            }
        }
    }
}

/// Write `FlatHvConfigBlob` bytes for Multiboot module 0.
pub fn write_flat_cfg_blob(root: &Path, cfg: &DemoConfig) -> Result<PathBuf, NestedError> {
    use hv::config::{FlatHvConfig, FlatHvConfigBlob};

    let flat = FlatHvConfig::from_demo(cfg);
    let blob = FlatHvConfigBlob::new(flat);
    let out = root.join("target/xtask-flat-hv-config.bin");
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            NestedError::Message(format!("mkdir {}: {e}", parent.display()))
        })?;
    }
    // SAFETY: POD repr(C); write exact struct bytes for the freestanding loader.
    let bytes = unsafe {
        core::slice::from_raw_parts(
            (&blob as *const FlatHvConfigBlob) as *const u8,
            core::mem::size_of::<FlatHvConfigBlob>(),
        )
    };
    std::fs::write(&out, bytes)
        .map_err(|e| NestedError::Message(format!("write {}: {e}", out.display())))?;
    Ok(out)
}

/// Build QEMU argv (program + args) from config only.
pub fn build_qemu_command(
    root: &Path,
    cfg: &DemoConfig,
    images: &NestedImages,
    _mode: NestedMode,
) -> Result<(PathBuf, Vec<String>), NestedError> {
    let qemu = PathBuf::from(&cfg.lab.qemu_bin);
    // Allow bare name on PATH.
    let qemu_bin = if qemu.components().count() == 1 {
        qemu
    } else if qemu.is_absolute() {
        qemu
    } else {
        root.join(qemu)
    };

    let mut args: Vec<String> = Vec::new();
    // Multiboot1 ADDR + KVM fails to enter the guest on several lab kernels
    // (empty serial / no debugcon). Default to TCG for a reliable Multiboot
    // bring-up; L1 inject smoke does not need VMXON. Override with HV_QEMU_ACCEL.
    let accel = std::env::var("HV_QEMU_ACCEL")
        .unwrap_or_else(|_| "tcg".into())
        .to_ascii_lowercase();
    match accel.as_str() {
        "kvm" => {
            args.push("-accel".into());
            args.push("kvm".into());
            args.push("-cpu".into());
            args.push("host,+vmx".into());
        }
        "kvm:tcg" | "auto" => {
            args.push("-accel".into());
            args.push("kvm:tcg".into());
            args.push("-cpu".into());
            args.push("host,+vmx".into());
        }
        _ => {
            if accel != "tcg" {
                eprintln!(
                    "xtask: unknown HV_QEMU_ACCEL={accel:?}, using tcg (Multiboot-safe default)"
                );
            }
            args.push("-accel".into());
            args.push("tcg".into());
            args.push("-cpu".into());
            args.push("qemu64".into());
            eprintln!(
                "xtask: using TCG Multiboot boot (set HV_QEMU_ACCEL=kvm to force KVM; \
                 see docs/HARNESS.md)"
            );
        }
    }
    args.push("-m".into());
    args.push(cfg.lab.memory_mb.to_string());
    args.push("-smp".into());
    args.push(cfg.lab.smp.to_string());
    args.push("-display".into());
    args.push("none".into());
    // Single serial on stdio (do not combine with -nographic).
    args.push("-serial".into());
    args.push("stdio".into());
    args.push("-monitor".into());
    args.push("none".into());
    args.push("-no-reboot".into());

    if cfg.lab.enable_iommu {
        args.push("-device".into());
        args.push("intel-iommu,intremap=on,device-iotlb=on".into());
    }

    if cfg.datapath.backend == VfBackend::Sriov {
        for bdf in [
            cfg.datapath.wan_vf_bdf.as_deref(),
            cfg.datapath.lan_vf_bdf.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            let bdf = bdf.trim();
            if bdf.is_empty() {
                continue;
            }
            // QEMU vfio-pci host device: address must match bound VF BDF from config.
            args.push("-device".into());
            args.push(format!("vfio-pci,host={bdf}"));
        }
    }

    // Raw Multiboot ADDR image (ELF64 is rejected by QEMU load_multiboot).
    let hv_kernel = ensure_hv_multiboot_bin(&images.hv)?;
    args.push("-kernel".into());
    args.push(hv_kernel.to_string_lossy().into_owned());

    // Multiboot modules: flat config, guest-wan, guest-lan (comma-separated initrd).
    let initrd = format!(
        "{},{},{}",
        images.flat_cfg.to_string_lossy(),
        images.guest_wan.to_string_lossy(),
        images.guest_lan.to_string_lossy()
    );
    args.push("-initrd".into());
    args.push(initrd);

    Ok((qemu_bin, args))
}

/// Load demonstrator config: `HV_DEMO_CONFIG`, else `config/demo.toml`, else example.
pub fn load_demo_config(root: &Path) -> Result<DemoConfig, NestedError> {
    if let Ok(p) = std::env::var("HV_DEMO_CONFIG") {
        return DemoConfig::load(&p).map_err(|e| NestedError::Message(format!("config {p}: {e}")));
    }
    let local = root.join("config/demo.toml");
    if local.is_file() {
        return DemoConfig::load(&local)
            .map_err(|e| NestedError::Message(format!("config {}: {e}", local.display())));
    }
    let example = root.join("config/demo.example.toml");
    eprintln!(
        "xtask: using {} (copy to config/demo.toml for lab overrides)",
        example.display()
    );
    DemoConfig::load(&example)
        .map_err(|e| NestedError::Message(format!("config {}: {e}", example.display())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use demo_config::DemoConfig;
    use std::sync::Mutex;

    /// Serialise tests that mutate `HV_TEST_VF_SYSFS_ROOT` (process-global env).
    static VF_SYSFS_LOCK: Mutex<()> = Mutex::new(());

    fn sriov_cfg(fallback: bool) -> DemoConfig {
        let toml = format!(
            r#"
[lab]
qemu_bin = "qemu-system-x86_64"
memory_mb = 512
smp = 2
enable_iommu = true

[datapath]
backend = "sriov"
allow_inject_fallback = {fallback}
wan_vf_bdf = "0000:de:10.0"
lan_vf_bdf = "0000:de:10.1"

[guests]
wan_memory_mb = 64
lan_memory_mb = 64
wan_vcpus = 1
lan_vcpus = 1

[ring]
capacity_pow2 = 64
frame_capacity = 256

[bench]
frame_size = 64
duration_secs = 1
bidirectional = true
target_gbps = 1.0

[hv]
gpa_window_base = 0x10000000
uart_io_port = 0x3F8

[paths]
hv_image = "target/x86_64-unknown-none/debug/hv"
guest_wan_image = "target/x86_64-unknown-none/debug/guest-wan"
guest_lan_image = "target/x86_64-unknown-none/debug/guest-lan"
"#
        );
        DemoConfig::parse_str(&toml).expect("sriov test cfg")
    }

    fn fake_vfio_tree(bdfs: &[&str]) -> PathBuf {
        let tmp = std::env::temp_dir().join(format!(
            "xtask-vf-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let drivers = tmp.join("drivers/vfio-pci");
        std::fs::create_dir_all(&drivers).unwrap();
        for bdf in bdfs {
            let dev = tmp.join(bdf);
            std::fs::create_dir_all(&dev).unwrap();
            std::os::unix::fs::symlink(&drivers, dev.join("driver")).unwrap();
        }
        tmp
    }

    #[test]
    fn resolve_sriov_fails_fast_without_vf_when_fallback_off() {
        let _guard = VF_SYSFS_LOCK.lock().unwrap();
        std::env::remove_var("HV_TEST_VF_SYSFS_ROOT");
        let mut cfg = sriov_cfg(false);
        let err = resolve_datapath_for_launch(&mut cfg).expect_err("no silent inject");
        let msg = err.to_string();
        assert!(msg.contains("allow_inject_fallback=false"));
        assert!(msg.contains("HARNESS"));
        assert_eq!(cfg.datapath.backend, VfBackend::Sriov);
    }

    #[test]
    fn resolve_sriov_fallback_rewrites_to_inject() {
        let _guard = VF_SYSFS_LOCK.lock().unwrap();
        std::env::remove_var("HV_TEST_VF_SYSFS_ROOT");
        let mut cfg = sriov_cfg(true);
        let backend = resolve_datapath_for_launch(&mut cfg).expect("fallback");
        assert_eq!(backend, VfBackend::Inject);
        assert_eq!(cfg.datapath.backend, VfBackend::Inject);
    }

    #[test]
    fn resolve_sriov_ready_when_vfio_bound() {
        let _guard = VF_SYSFS_LOCK.lock().unwrap();
        let tmp = fake_vfio_tree(&["0000:de:10.0", "0000:de:10.1"]);
        std::env::set_var("HV_TEST_VF_SYSFS_ROOT", &tmp);
        let mut cfg = sriov_cfg(false);
        let backend = resolve_datapath_for_launch(&mut cfg);
        std::env::remove_var("HV_TEST_VF_SYSFS_ROOT");
        let _ = std::fs::remove_dir_all(&tmp);
        assert_eq!(backend.expect("ready"), VfBackend::Sriov);
        assert_eq!(cfg.datapath.backend, VfBackend::Sriov);
    }

    #[test]
    fn resolve_sriov_rejects_unbound_device_without_fallback() {
        let _guard = VF_SYSFS_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "xtask-vf-unbound-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        for bdf in ["0000:de:10.0", "0000:de:10.1"] {
            std::fs::create_dir_all(tmp.join(bdf)).unwrap();
        }
        std::env::set_var("HV_TEST_VF_SYSFS_ROOT", &tmp);
        let mut cfg = sriov_cfg(false);
        let err = resolve_datapath_for_launch(&mut cfg).expect_err("unbound");
        std::env::remove_var("HV_TEST_VF_SYSFS_ROOT");
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(err.to_string().contains("vfio-pci"));
        assert_eq!(cfg.datapath.backend, VfBackend::Sriov);
    }
}
