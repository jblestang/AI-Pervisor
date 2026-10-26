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

/// Sysfs presence check for a PCI BDF (`0000:bb:dd.f`).
pub fn vf_sysfs_present(bdf: &str) -> bool {
    let bdf = bdf.trim();
    if bdf.is_empty() {
        return false;
    }
    Path::new("/sys/bus/pci/devices").join(bdf).is_dir()
}

/// Apply Inject fallback policy when Sriov VFs are missing.
///
/// Returns the backend that will actually be used (may rewrite `cfg` in place).
pub fn resolve_datapath_for_launch(cfg: &mut DemoConfig) -> Result<VfBackend, NestedError> {
    match cfg.datapath.backend {
        VfBackend::Inject => Ok(VfBackend::Inject),
        VfBackend::Sriov => {
            let wan = cfg.datapath.wan_vf_bdf.as_deref().unwrap_or("").trim();
            let lan = cfg.datapath.lan_vf_bdf.as_deref().unwrap_or("").trim();
            let wan_ok = vf_sysfs_present(wan);
            let lan_ok = vf_sysfs_present(lan);
            if wan_ok && lan_ok {
                return Ok(VfBackend::Sriov);
            }
            if cfg.datapath.allow_inject_fallback {
                eprintln!(
                    "xtask: Sriov VF missing (wan_present={wan_ok} lan_present={lan_ok}); \
                     allow_inject_fallback=true → using Inject"
                );
                cfg.datapath.backend = VfBackend::Inject;
                Ok(VfBackend::Inject)
            } else {
                Err(NestedError::Message(format!(
                    "datapath.backend=sriov but VF sysfs missing \
                     (wan={wan:?} present={wan_ok}, lan={lan:?} present={lan_ok}) \
                     and allow_inject_fallback=false.\n  \
                     hint: bind VFs per docs/HARNESS.md, or set datapath.backend=inject \
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
