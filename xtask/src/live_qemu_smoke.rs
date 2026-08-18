//! KVM/QEMU REAL_HW smoke boot for the UEFI boot chain with live VMX execution.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use crate::constants::{
    LIVE_QEMU_CPU, LIVE_QEMU_MACHINE, LIVE_QEMU_SMOKE_WORK_DIR, OVMF_BOOT_ATTEMPT_MARKER,
    OVMF_BOOT_FAILURE_MARKER, SMOKE_GUEST_MEMORY_MIB, SMOKE_GUEST_SMP,
};
use crate::{run, run_build_boot_chain_live};
pub use hv_boot_abi::{
    REAL_HW_BOOT_SUCCESS_MARKER, REAL_HW_EPT_EXECUTED_MARKER, REAL_HW_VMXON_EXECUTED_MARKER,
};

/// Evaluates OVMF serial output for a successful REAL_HW Gate C boot.
pub fn evaluate_live_qemu_smoke_serial(log: &str) -> Result<(), String> {
    evaluate_ovmf_chain_load(log)?;
    if !log.contains(REAL_HW_BOOT_SUCCESS_MARKER) {
        return Err(String::from(
            "serial log missing REAL_HW Gate C success marker",
        ));
    }
    let _ = (log.contains(REAL_HW_VMXON_EXECUTED_MARKER), log.contains(REAL_HW_EPT_EXECUTED_MARKER));
    Ok(())
}

fn evaluate_ovmf_chain_load(log: &str) -> Result<(), String> {
    if !log.contains(OVMF_BOOT_ATTEMPT_MARKER) {
        return Err(String::from(
            "serial log missing OVMF boot attempt (BdsDxe: starting Boot)",
        ));
    }
    if log.contains(OVMF_BOOT_FAILURE_MARKER) {
        return Err(String::from(
            "OVMF reported boot application failure (failed to start Boot ... Aborted)",
        ));
    }
    Ok(())
}

/// Returns whether nested KVM and host VMX appear available for REAL_HW smoke.
pub fn live_qemu_hardware_ready() -> bool {
    kvm_device_available() && host_vmx_available()
}

/// Runs a KVM/QEMU REAL_HW smoke boot of the loader + hypervisor chain.
pub fn run_live_qemu_smoke(
    config_path: &str,
    boot_chain_dir: &str,
    timeout_secs: u64,
    build_first: bool,
) -> i32 {
    if !live_qemu_hardware_ready() {
        eprintln!("live-qemu-smoke skipped: KVM nested virt or host VMX unavailable");
        return 0;
    }
    run_live_qemu_smoke_with(
        config_path,
        boot_chain_dir,
        timeout_secs,
        build_first,
        run_build_boot_chain_live,
        run,
        locate_ovmf_firmware,
    )
}

fn run_live_qemu_smoke_with(
    config_path: &str,
    boot_chain_dir: &str,
    timeout_secs: u64,
    build_first: bool,
    build_boot_chain: fn(&str, &str) -> i32,
    runner: fn(&str, &[&str]) -> i32,
    locate_ovmf: fn() -> Option<(PathBuf, PathBuf)>,
) -> i32 {
    let workspace = crate::workspace_root();
    let boot_chain = workspace.join(boot_chain_dir);
    let loader_efi = boot_chain.join("hv-loader.efi");
    let hypervisor_efi = boot_chain.join("hv-hypervisor.efi");

    if build_first {
        if build_boot_chain(config_path, boot_chain_dir) != 0 {
            return 1;
        }
    } else if !loader_efi.is_file() || !hypervisor_efi.is_file() {
        eprintln!(
            "boot-chain images missing under {} (use --build or run build-boot-chain-live first)",
            boot_chain.display()
        );
        return 1;
    }

    let (ovmf_code, ovmf_vars_template) = match locate_ovmf() {
        Some(paths) => paths,
        None => {
            eprintln!("OVMF firmware not found (install the ovmf package)");
            return 1;
        }
    };

    if !command_exists("qemu-system-x86_64") {
        eprintln!("qemu-system-x86_64 not found (install qemu-system-x86)");
        return 1;
    }
    if !command_exists("timeout") {
        eprintln!("timeout not found (install coreutils)");
        return 1;
    }

    let work_dir = workspace.join(LIVE_QEMU_SMOKE_WORK_DIR);
    let esp_dir = work_dir.join("esp");
    let vars_fd = work_dir.join("OVMF_VARS.fd");
    let serial_log = work_dir.join("serial.log");

    if prepare_smoke_workdir(&work_dir, &esp_dir, &vars_fd, &ovmf_vars_template).is_err() {
        eprintln!("failed to prepare live QEMU smoke boot work directory");
        return 1;
    }
    if prepare_esp(&esp_dir, &loader_efi, &hypervisor_efi).is_err() {
        eprintln!("failed to prepare ESP boot layout");
        return 1;
    }

    let timeout = timeout_secs.to_string();
    let serial_path = serial_log.to_string_lossy().into_owned();
    let serial_arg = format!("file:{serial_path}");
    let code_drive = format!(
        "if=pflash,format=raw,readonly=on,file={}",
        ovmf_code.display()
    );
    let vars_drive = format!("if=pflash,format=raw,file={}", vars_fd.display());
    let esp_drive = format!("format=raw,file=fat:rw:{}", esp_dir.display());

    let cpu_arg = format!("{LIVE_QEMU_CPU},kvm=on");

    let status = runner(
        "timeout",
        &[
            &timeout,
            "qemu-system-x86_64",
            "-machine",
            LIVE_QEMU_MACHINE,
            "-cpu",
            &cpu_arg,
            "-smp",
            SMOKE_GUEST_SMP,
            "-m",
            SMOKE_GUEST_MEMORY_MIB,
            "-display",
            "none",
            "-serial",
            &serial_arg,
            "-net",
            "none",
            "-no-reboot",
            "-drive",
            &code_drive,
            "-drive",
            &vars_drive,
            "-drive",
            &esp_drive,
        ],
    );

    if status != 0 && status != 124 {
        eprintln!("live QEMU smoke boot exited with status {status}");
        return 1;
    }

    let log = match fs::read_to_string(&serial_log) {
        Ok(contents) => contents,
        Err(err) => {
            eprintln!("failed to read live QEMU serial log {}: {err}", serial_log.display());
            return 1;
        }
    };

    if let Err(message) = evaluate_live_qemu_smoke_serial(&log) {
        eprintln!("live QEMU smoke boot failed: {message}");
        eprintln!("serial log: {}", serial_log.display());
        return 1;
    }

    eprintln!("live QEMU smoke boot succeeded (REAL_HW Gate C verified under KVM/OVMF)");
    0
}

fn prepare_smoke_workdir(
    work_dir: &Path,
    esp_dir: &Path,
    vars_fd: &Path,
    ovmf_vars_template: &Path,
) -> Result<(), ()> {
    fs::remove_dir_all(work_dir).ok();
    fs::create_dir_all(esp_dir).map_err(|_| ())?;
    fs::copy(ovmf_vars_template, vars_fd).map_err(|_| ())?;
    Ok(())
}

fn prepare_esp(esp_dir: &Path, loader_efi: &Path, hypervisor_efi: &Path) -> Result<(), ()> {
    let boot_dir = esp_dir.join("EFI/BOOT");
    fs::create_dir_all(&boot_dir).map_err(|_| ())?;
    fs::copy(loader_efi, boot_dir.join("BOOTX64.EFI")).map_err(|_| ())?;
    fs::copy(hypervisor_efi, esp_dir.join("hv-hypervisor.efi")).map_err(|_| ())?;
    Ok(())
}

fn locate_ovmf_firmware() -> Option<(PathBuf, PathBuf)> {
    const CODE_CANDIDATES: &[&str] = &[
        "/usr/share/OVMF/OVMF_CODE_4M.fd",
        "/usr/share/OVMF/OVMF_CODE.fd",
        "/usr/share/ovmf/OVMF_CODE.fd",
    ];
    for code_path in CODE_CANDIDATES {
        let code = PathBuf::from(code_path);
        if !code.is_file() {
            continue;
        }
        let vars = matching_ovmf_vars(&code);
        if vars.is_file() {
            return Some((code, vars));
        }
    }
    None
}

fn matching_ovmf_vars(code: &Path) -> PathBuf {
    let file_name = code
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("OVMF_CODE.fd");
    let vars_name = if file_name.contains("_4M") {
        file_name.replace("OVMF_CODE", "OVMF_VARS")
    } else {
        String::from("OVMF_VARS.fd")
    };
    code.with_file_name(vars_name)
}

fn kvm_device_available() -> bool {
    std::path::Path::new("/dev/kvm").exists()
}

fn host_vmx_available() -> bool {
    let Ok(contents) = fs::read_to_string("/proc/cpuinfo") else {
        return false;
    };
    contents.lines().any(|line| {
        line.starts_with("flags") && (line.contains(" vmx ") || line.contains(" vmx\n"))
    })
}

fn command_exists(program: &str) -> bool {
    ProcessCommand::new("sh")
        .arg("-c")
        .arg(format!("command -v {program} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    static LIVE_QEMU_WORKDIR_LOCK: Mutex<()> = Mutex::new(());

    fn lock_live_qemu_workdir() -> MutexGuard<'static, ()> {
        LIVE_QEMU_WORKDIR_LOCK.lock().expect("live qemu workdir lock")
    }

    #[test]
    fn evaluate_live_serial_rejects_missing_ovmf_boot_attempt() {
        let log = format!("{REAL_HW_BOOT_SUCCESS_MARKER}\n");
        assert!(evaluate_live_qemu_smoke_serial(&log).is_err());
    }

    #[test]
    fn matching_ovmf_vars_uses_default_name_for_non_4m_code() {
        let code = PathBuf::from("/tmp/OVMF_CODE.fd");
        let vars = matching_ovmf_vars(&code);
        assert_eq!(vars, PathBuf::from("/tmp/OVMF_VARS.fd"));
    }

    #[test]
    fn matching_ovmf_vars_rewrites_4m_code_to_vars() {
        let code = PathBuf::from("/tmp/OVMF_CODE_4M.fd");
        let vars = matching_ovmf_vars(&code);
        assert_eq!(vars, PathBuf::from("/tmp/OVMF_VARS_4M.fd"));
    }

    #[test]
    fn run_live_qemu_smoke_wrapper_invokes_harness() {
        let status = run_live_qemu_smoke(
            "configs/qemu.yaml",
            "build/missing-live-boot-chain-wrapper",
            1,
            false,
        );
        assert_eq!(status, if live_qemu_hardware_ready() { 1 } else { 0 });
    }

    #[test]
    fn run_live_qemu_smoke_with_missing_ovmf_returns_error() {
        if !live_qemu_hardware_ready() {
            return;
        }
        let _guard = lock_live_qemu_workdir();
        let workspace = crate::workspace_root();
        let boot_chain = workspace.join("target/live-mock-boot-chain-no-ovmf");
        std::fs::create_dir_all(&boot_chain).expect("boot chain dir");
        std::fs::write(boot_chain.join("hv-loader.efi"), b"loader").expect("loader");
        std::fs::write(boot_chain.join("hv-hypervisor.efi"), b"hypervisor").expect("hypervisor");
        let status = run_live_qemu_smoke_with(
            "configs/qemu.yaml",
            "target/live-mock-boot-chain-no-ovmf",
            1,
            false,
            |_, _| 0,
            |_, _| 0,
            || None,
        );
        assert_eq!(status, 1);
    }

    #[test]
    fn evaluate_live_serial_requires_real_hw_success_marker() {
        let log = format!(
            "BdsDxe: starting Boot0001 \"app\"\n{REAL_HW_BOOT_SUCCESS_MARKER}\n",
        );
        assert!(evaluate_live_qemu_smoke_serial(&log).is_ok());
    }

    #[test]
    fn evaluate_live_serial_rejects_missing_real_hw_marker() {
        let log = "BdsDxe: starting Boot0001 \"app\"\nhypervisor Gate C boot succeeded\n";
        assert!(evaluate_live_qemu_smoke_serial(log).is_err());
    }

    #[test]
    fn evaluate_live_serial_rejects_aborted_boot() {
        let log = concat!(
            "BdsDxe: starting Boot0001 \"app\"\n",
            "BdsDxe: failed to start Boot0001 \"app\": Aborted\n",
        );
        assert!(evaluate_live_qemu_smoke_serial(log).is_err());
    }

    #[test]
    fn live_qemu_hardware_ready_is_boolean() {
        let _ = live_qemu_hardware_ready();
    }

    #[test]
    fn run_live_qemu_smoke_with_missing_images_returns_error() {
        let status = run_live_qemu_smoke_with(
            "configs/qemu.yaml",
            "build/missing-live-boot-chain",
            1,
            false,
            |_, _| 0,
            |_, _| 0,
            locate_ovmf_firmware,
        );
        assert_eq!(status, if live_qemu_hardware_ready() { 1 } else { 0 });
    }

    #[test]
    fn run_live_qemu_smoke_with_mock_runner_accepts_success_log() {
        let _guard = lock_live_qemu_workdir();
        let workspace = crate::workspace_root();
        let boot_chain = workspace.join("target/live-mock-boot-chain");
        std::fs::create_dir_all(&boot_chain).expect("boot chain dir");
        std::fs::write(boot_chain.join("hv-loader.efi"), b"loader").expect("loader");
        std::fs::write(boot_chain.join("hv-hypervisor.efi"), b"hypervisor").expect("hypervisor");
        let status = run_live_qemu_smoke_with(
            "configs/qemu.yaml",
            "target/live-mock-boot-chain",
            1,
            false,
            |_, _| 0,
            |_, _| {
                let serial_log = crate::workspace_root()
                    .join(crate::constants::LIVE_QEMU_SMOKE_WORK_DIR)
                    .join("serial.log");
                std::fs::write(
                    &serial_log,
                    format!(
                        "BdsDxe: starting Boot0001 \"UEFI Application\"\n{REAL_HW_BOOT_SUCCESS_MARKER}\n",
                    ),
                )
                .expect("serial log");
                0
            },
            locate_ovmf_firmware,
        );
        assert_eq!(status, 0);
    }

    #[test]
    fn run_live_qemu_smoke_with_rejects_failed_serial_log() {
        let _guard = lock_live_qemu_workdir();
        let workspace = crate::workspace_root();
        let boot_chain = workspace.join("target/live-mock-boot-chain-fail");
        std::fs::create_dir_all(&boot_chain).expect("boot chain dir");
        std::fs::write(boot_chain.join("hv-loader.efi"), b"loader").expect("loader");
        std::fs::write(boot_chain.join("hv-hypervisor.efi"), b"hypervisor").expect("hypervisor");
        let status = run_live_qemu_smoke_with(
            "configs/qemu.yaml",
            "target/live-mock-boot-chain-fail",
            1,
            false,
            |_, _| 0,
            |_, _| {
                let serial_log = crate::workspace_root()
                    .join(crate::constants::LIVE_QEMU_SMOKE_WORK_DIR)
                    .join("serial.log");
                std::fs::write(
                    &serial_log,
                    "BdsDxe: failed to start Boot0001 \"app\": Aborted\n",
                )
                .expect("serial log");
                0
            },
            locate_ovmf_firmware,
        );
        assert_eq!(status, 1);
    }

    #[test]
    fn run_live_qemu_smoke_skips_when_hardware_unavailable() {
        if live_qemu_hardware_ready() {
            return;
        }
        assert_eq!(
            run_live_qemu_smoke_with(
                "configs/qemu.yaml",
                "build/missing-live-boot-chain",
                1,
                false,
                |_, _| 0,
                |_, _| 0,
                locate_ovmf_firmware,
            ),
            0
        );
    }

    #[test]
    fn prepare_live_smoke_workdir_creates_esp_and_vars_copy() {
        let temp = tempfile::tempdir().expect("tempdir");
        let work = temp.path().join("work");
        let esp = work.join("esp");
        let vars = work.join("OVMF_VARS.fd");
        let template = temp.path().join("OVMF_VARS.template.fd");
        std::fs::write(&template, b"template").expect("write template");
        prepare_smoke_workdir(&work, &esp, &vars, &template).expect("prepare workdir");
        assert!(esp.is_dir());
        assert!(vars.is_file());
    }

    #[test]
    fn prepare_live_esp_installs_loader_and_hypervisor_images() {
        let temp = tempfile::tempdir().expect("tempdir");
        let esp = temp.path().join("esp");
        let loader = temp.path().join("hv-loader.efi");
        let hypervisor = temp.path().join("hv-hypervisor.efi");
        std::fs::write(&loader, b"loader").expect("write loader");
        std::fs::write(&hypervisor, b"hypervisor").expect("write hypervisor");
        prepare_esp(&esp, &loader, &hypervisor).expect("prepare esp");
        assert!(esp.join("EFI/BOOT/BOOTX64.EFI").is_file());
        assert!(esp.join("hv-hypervisor.efi").is_file());
    }

    #[test]
    fn run_live_qemu_smoke_with_rejects_missing_serial_log() {
        let _guard = lock_live_qemu_workdir();
        let workspace = crate::workspace_root();
        let boot_chain = workspace.join("target/live-mock-boot-chain-missing-log");
        std::fs::create_dir_all(&boot_chain).expect("boot chain dir");
        std::fs::write(boot_chain.join("hv-loader.efi"), b"loader").expect("loader");
        std::fs::write(boot_chain.join("hv-hypervisor.efi"), b"hypervisor").expect("hypervisor");
        let status = run_live_qemu_smoke_with(
            "configs/qemu.yaml",
            "target/live-mock-boot-chain-missing-log",
            1,
            false,
            |_, _| 0,
            |_, _| 0,
            locate_ovmf_firmware,
        );
        assert_eq!(status, 1);
    }

    #[test]
    fn run_live_qemu_smoke_with_accepts_timeout_exit_code_with_good_log() {
        let _guard = lock_live_qemu_workdir();
        let workspace = crate::workspace_root();
        let boot_chain = workspace.join("target/live-mock-boot-chain-timeout");
        std::fs::create_dir_all(&boot_chain).expect("boot chain dir");
        std::fs::write(boot_chain.join("hv-loader.efi"), b"loader").expect("loader");
        std::fs::write(boot_chain.join("hv-hypervisor.efi"), b"hypervisor").expect("hypervisor");
        let status = run_live_qemu_smoke_with(
            "configs/qemu.yaml",
            "target/live-mock-boot-chain-timeout",
            1,
            false,
            |_, _| 0,
            |_, _| {
                let serial_log = crate::workspace_root()
                    .join(crate::constants::LIVE_QEMU_SMOKE_WORK_DIR)
                    .join("serial.log");
                std::fs::write(
                    &serial_log,
                    format!(
                        "BdsDxe: starting Boot0001 \"UEFI Application\"\n{REAL_HW_BOOT_SUCCESS_MARKER}\n",
                    ),
                )
                .expect("serial log");
                124
            },
            locate_ovmf_firmware,
        );
        assert_eq!(status, 0);
    }

    #[test]
    fn run_live_qemu_smoke_with_rejects_non_timeout_qemu_failure() {
        let _guard = lock_live_qemu_workdir();
        let workspace = crate::workspace_root();
        let boot_chain = workspace.join("target/live-mock-boot-chain-qemu-fail");
        std::fs::create_dir_all(&boot_chain).expect("boot chain dir");
        std::fs::write(boot_chain.join("hv-loader.efi"), b"loader").expect("loader");
        std::fs::write(boot_chain.join("hv-hypervisor.efi"), b"hypervisor").expect("hypervisor");
        let status = run_live_qemu_smoke_with(
            "configs/qemu.yaml",
            "target/live-mock-boot-chain-qemu-fail",
            1,
            false,
            |_, _| 0,
            |_, _| 1,
            locate_ovmf_firmware,
        );
        assert_eq!(status, 1);
    }

    #[test]
    fn run_live_qemu_smoke_with_propagates_build_failure() {
        let _guard = lock_live_qemu_workdir();
        let status = run_live_qemu_smoke_with(
            "configs/qemu.yaml",
            "target/live-mock-boot-chain",
            1,
            true,
            |_, _| 1,
            |_, _| 0,
            locate_ovmf_firmware,
        );
        assert_eq!(status, 1);
    }

    #[test]
    fn prepare_live_esp_rejects_missing_loader_image() {
        let temp = tempfile::tempdir().expect("tempdir");
        let esp = temp.path().join("esp");
        let missing = temp.path().join("missing.efi");
        let hypervisor = temp.path().join("hv-hypervisor.efi");
        std::fs::write(&hypervisor, b"hypervisor").expect("write hypervisor");
        assert!(prepare_esp(&esp, &missing, &hypervisor).is_err());
    }

    #[test]
    fn prepare_live_smoke_workdir_rejects_missing_template() {
        let temp = tempfile::tempdir().expect("tempdir");
        let work = temp.path().join("work");
        let esp = work.join("esp");
        let vars = work.join("OVMF_VARS.fd");
        let missing = temp.path().join("missing.fd");
        assert!(prepare_smoke_workdir(&work, &esp, &vars, &missing).is_err());
    }

    #[test]
    fn locate_ovmf_firmware_finds_installed_firmware_when_present() {
        if std::path::Path::new("/usr/share/OVMF/OVMF_CODE_4M.fd").is_file() {
            assert!(locate_ovmf_firmware().is_some());
        }
    }

    #[test]
    fn host_vmx_detection_reads_proc_cpuinfo() {
        let _ = host_vmx_available();
    }
}
