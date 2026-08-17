//! OVMF/QEMU smoke boot for the UEFI boot chain.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use crate::constants::OVMF_SMOKE_WORK_DIR;
use crate::{run, run_build_boot_chain};

const OVMF_BOOT_ATTEMPT_MARKER: &str = "BdsDxe: starting Boot";
const OVMF_BOOT_FAILURE_MARKER: &str = "failed to start Boot";

/// Evaluates OVMF serial output for a successful boot-chain handoff.
pub fn evaluate_ovmf_smoke_boot_serial(log: &str) -> Result<(), String> {
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

/// Runs an OVMF/QEMU smoke boot of the loader + hypervisor chain.
pub fn run_ovmf_smoke_boot(
    config_path: &str,
    boot_chain_dir: &str,
    timeout_secs: u64,
    build_first: bool,
) -> i32 {
    run_ovmf_smoke_boot_with(
        config_path,
        boot_chain_dir,
        timeout_secs,
        build_first,
        run_build_boot_chain,
        run,
    )
}

fn run_ovmf_smoke_boot_with(
    config_path: &str,
    boot_chain_dir: &str,
    timeout_secs: u64,
    build_first: bool,
    build_boot_chain: fn(&str, &str) -> i32,
    runner: fn(&str, &[&str]) -> i32,
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
            "boot-chain images missing under {} (use --build or run build-boot-chain first)",
            boot_chain.display()
        );
        return 1;
    }

    let (ovmf_code, ovmf_vars_template) = match locate_ovmf_firmware() {
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

    let work_dir = workspace.join(OVMF_SMOKE_WORK_DIR);
    let esp_dir = work_dir.join("esp");
    let vars_fd = work_dir.join("OVMF_VARS.fd");
    let serial_log = work_dir.join("serial.log");

    if prepare_smoke_workdir(&work_dir, &esp_dir, &vars_fd, &ovmf_vars_template).is_err() {
        eprintln!("failed to prepare OVMF smoke boot work directory");
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

    let status = runner(
        "timeout",
        &[
            &timeout,
            "qemu-system-x86_64",
            "-machine",
            "q35,accel=tcg",
            "-cpu",
            "max",
            "-m",
            "4096",
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
        eprintln!("qemu smoke boot exited with status {status}");
        return 1;
    }

    let log = match fs::read_to_string(&serial_log) {
        Ok(contents) => contents,
        Err(err) => {
            eprintln!("failed to read OVMF serial log {}: {err}", serial_log.display());
            return 1;
        }
    };

    if let Err(message) = evaluate_ovmf_smoke_boot_serial(&log) {
        eprintln!("OVMF smoke boot failed: {message}");
        eprintln!("serial log: {}", serial_log.display());
        return 1;
    }

    eprintln!("OVMF smoke boot succeeded (loader chain-load verified under firmware)");
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

    #[test]
    fn evaluate_serial_accepts_successful_boot_log() {
        let log = concat!(
            "BdsDxe: loading Boot0001 \"UEFI QEMU HARDDISK QM00001 \"\n",
            "BdsDxe: starting Boot0001 \"UEFI QEMU HARDDISK QM00001 \"\n",
            "BdsDxe: loading Boot0000 \"UiApp\"\n",
            "BdsDxe: starting Boot0000 \"UiApp\"\n",
        );
        assert!(evaluate_ovmf_smoke_boot_serial(log).is_ok());
    }

    #[test]
    fn evaluate_serial_rejects_aborted_boot_log() {
        let log = concat!(
            "BdsDxe: starting Boot0001 \"UEFI QEMU HARDDISK QM00001 \"\n",
            "BdsDxe: failed to start Boot0001 \"UEFI QEMU HARDDISK QM00001 \": Aborted\n",
        );
        assert!(evaluate_ovmf_smoke_boot_serial(log).is_err());
    }

    #[test]
    fn evaluate_serial_rejects_missing_boot_attempt() {
        assert!(evaluate_ovmf_smoke_boot_serial("no boot here").is_err());
    }

    #[test]
    fn matching_ovmf_vars_uses_4m_pairing() {
        let code = PathBuf::from("/usr/share/OVMF/OVMF_CODE_4M.fd");
        assert_eq!(
            matching_ovmf_vars(&code),
            PathBuf::from("/usr/share/OVMF/OVMF_VARS_4M.fd")
        );
    }
}
