//! `cargo xtask coverage` — llvm-cov gate with explicit TCB allowlist.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde::Deserialize;

const ALLOWLIST_REL: &str = "xtask/coverage-allowlist.toml";
const REPORT_JSON_REL: &str = "coverage/llvm-cov-summary.json";

#[derive(Debug, Deserialize)]
struct Allowlist {
    threshold_lines_percent: f64,
    packages: Vec<String>,
    #[serde(default)]
    exclude: Vec<Exclude>,
}

#[derive(Debug, Deserialize)]
struct Exclude {
    path_regex: String,
    justification: String,
}

#[derive(Debug, Deserialize)]
struct LlvmCovExport {
    data: Vec<LlvmCovData>,
}

#[derive(Debug, Deserialize)]
struct LlvmCovData {
    totals: LlvmCovTotals,
}

#[derive(Debug, Deserialize)]
struct LlvmCovTotals {
    lines: LlvmCovLines,
}

#[derive(Debug, Deserialize)]
struct LlvmCovLines {
    count: u64,
    covered: u64,
    percent: f64,
}

fn resolve_llvm_cov() -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var("CARGO_LLVM_COV") {
        let path = PathBuf::from(p);
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!("CARGO_LLVM_COV set but not a file: {}", path.display()));
    }
    // Prefer cargo subcommand discovery via `cargo llvm-cov`.
    Ok(PathBuf::from("cargo"))
}

pub fn cmd_coverage(root: &Path) -> ExitCode {
    let allow_path = root.join(ALLOWLIST_REL);
    let text = match fs::read_to_string(&allow_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("xtask coverage: cannot read {}: {e}", allow_path.display());
            return ExitCode::FAILURE;
        }
    };
    let allow: Allowlist = match toml::from_str(&text) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("xtask coverage: invalid allowlist {}: {e}", allow_path.display());
            return ExitCode::FAILURE;
        }
    };
    if allow.packages.is_empty() {
        eprintln!("xtask coverage: allowlist packages list is empty");
        return ExitCode::FAILURE;
    }
    for ex in &allow.exclude {
        if ex.justification.trim().is_empty() {
            eprintln!(
                "xtask coverage: exclusion `{}` missing justification (no silent exclusions)",
                ex.path_regex
            );
            return ExitCode::FAILURE;
        }
        eprintln!(
            "xtask coverage: exclude `{}` — {}",
            ex.path_regex, ex.justification
        );
    }

    let report_path = root.join(REPORT_JSON_REL);
    if let Some(parent) = report_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("xtask coverage: cannot create {}: {e}", parent.display());
            return ExitCode::FAILURE;
        }
    }

    let llvm_cov = match resolve_llvm_cov() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("xtask coverage: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut args: Vec<String> = Vec::new();
    if llvm_cov.file_name().and_then(|s| s.to_str()) == Some("cargo") {
        args.push("llvm-cov".into());
    }
    args.push("--json".into());
    args.push("--summary-only".into());
    args.push("--output-path".into());
    args.push(report_path.display().to_string());
    for pkg in &allow.packages {
        args.push("-p".into());
        args.push(pkg.clone());
    }
    // Always ignore registry / rustc internals from the percentage.
    let mut ignore = String::from("(/registry/src/|/rustc/|/rustlib/)");
    for ex in &allow.exclude {
        ignore.push('|');
        ignore.push_str(&ex.path_regex);
    }
    args.push("--ignore-filename-regex".into());
    args.push(ignore);

    eprintln!(
        "xtask coverage: running {} {}",
        llvm_cov.display(),
        args.join(" ")
    );
    let status = Command::new(&llvm_cov)
        .args(&args)
        .current_dir(root)
        .status();
    let status = match status {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "xtask coverage: failed to spawn {} ({e}).\n  \
                 hint: install cargo-llvm-cov and rustup component llvm-tools-preview",
                llvm_cov.display()
            );
            return ExitCode::FAILURE;
        }
    };
    if !status.success() {
        eprintln!("xtask coverage: cargo llvm-cov failed with {status}");
        return ExitCode::FAILURE;
    }

    let report_text = match fs::read_to_string(&report_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!(
                "xtask coverage: missing report {}: {e}",
                report_path.display()
            );
            return ExitCode::FAILURE;
        }
    };
    let report: LlvmCovExport = match serde_json::from_str(&report_text) {
        Ok(r) => r,
        Err(e) => {
            // Fallback: cargo-llvm-cov wraps export; try reading percent via regex.
            eprintln!("xtask coverage: JSON parse via serde failed ({e}); trying fallback");
            return parse_percent_fallback(&report_text, allow.threshold_lines_percent, &report_path);
        }
    };
    let Some(data) = report.data.first() else {
        eprintln!("xtask coverage: empty llvm-cov data in {}", report_path.display());
        return ExitCode::FAILURE;
    };
    let lines = &data.totals.lines;
    let pct = lines.percent;
    println!(
        "xtask coverage: line coverage {:.4}% ({}/{} lines) [threshold {:.1}%]",
        pct, lines.covered, lines.count, allow.threshold_lines_percent
    );
    println!("xtask coverage: report {}", report_path.display());
    if pct + f64::EPSILON < allow.threshold_lines_percent {
        eprintln!(
            "xtask coverage: FAILED — {:.4}% < {:.1}%",
            pct, allow.threshold_lines_percent
        );
        ExitCode::FAILURE
    } else {
        println!("xtask coverage: ok");
        ExitCode::SUCCESS
    }
}

fn parse_percent_fallback(text: &str, threshold: f64, report_path: &Path) -> ExitCode {
    // Minimal extract: "\"lines\": { ... \"percent\": N }"
    let key = "\"percent\":";
    let mut last = None;
    for (idx, _) in text.match_indices(key) {
        let rest = &text[idx + key.len()..];
        let num: String = rest
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        if let Ok(v) = num.parse::<f64>() {
            last = Some(v);
        }
    }
    let Some(pct) = last else {
        eprintln!("xtask coverage: could not parse line percent from report");
        return ExitCode::FAILURE;
    };
    println!(
        "xtask coverage: line coverage {:.4}% [threshold {:.1}%] (fallback parse)",
        pct, threshold
    );
    println!("xtask coverage: report {}", report_path.display());
    if pct + f64::EPSILON < threshold {
        eprintln!("xtask coverage: FAILED — {:.4}% < {:.1}%", pct, threshold);
        ExitCode::FAILURE
    } else {
        println!("xtask coverage: ok");
        ExitCode::SUCCESS
    }
}
