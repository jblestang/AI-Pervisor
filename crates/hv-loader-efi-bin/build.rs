//! Embeds the compiled configuration digest into the loader image.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("manifest dir");
    let default_path = Path::new(&manifest_dir).join("../../build/config.sha256");
    let digest_path = env::var("HV_CONFIG_DIGEST_PATH").map(PathBuf::from).unwrap_or(default_path);
    let digest_path = if digest_path.is_absolute() {
        digest_path
    } else {
        Path::new(&manifest_dir)
            .join("../..")
            .join(digest_path)
    };

    let hex = fs::read_to_string(&digest_path).unwrap_or_else(|err| {
        panic!(
            "failed to read config digest at {}: {err}. Run `cargo xtask config generate` first.",
            digest_path.display()
        )
    });
    let bytes = hex::decode(hex.trim()).unwrap_or_else(|err| {
        panic!(
            "invalid config digest hex in {}: {err}",
            digest_path.display()
        )
    });
    if bytes.len() != 32 {
        panic!(
            "expected 32-byte config digest in {}, found {} bytes",
            digest_path.display(),
            bytes.len()
        );
    }

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR");
    let digest_rs = Path::new(&out_dir).join("config_digest.rs");
    let mut rendered = String::from("pub const CONFIG_DIGEST: [u8; 32] = [");
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            rendered.push_str(", ");
        }
        rendered.push_str(&format!("0x{byte:02X}"));
    }
    rendered.push_str("];\n");
    fs::write(&digest_rs, rendered).expect("write digest rs");
    println!("cargo:rerun-if-env-changed=HV_CONFIG_DIGEST_PATH");
    println!("cargo:rerun-if-changed={}", digest_path.display());
}
