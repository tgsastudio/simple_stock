// Build script to embed the Windows application icon (.ico) and version info into simple_stock.exe.
// Supports both MinGW/GNU (windres) and MSVC (rc.exe via PATH or Windows SDK).
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn find_rc_exe() -> Option<PathBuf> {
    if Command::new("rc.exe").arg("/?").output().is_ok() {
        return Some(PathBuf::from("rc.exe"));
    }
    let kits_bin = Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin");
    if let Ok(entries) = fs::read_dir(kits_bin) {
        let mut versions: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_dir())
            .collect();
        versions.sort();
        versions.reverse();
        for ver_dir in versions {
            let candidate = ver_dir.join("x64").join("rc.exe");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

fn main() {
    println!("cargo:rerun-if-changed=assets/app.rc");
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=assets/app.manifest");

    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "windows" {
        return;
    }

    let out_dir = env::var("OUT_DIR").unwrap();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();

    if target_env == "msvc" {
        let rc_exe = find_rc_exe()
            .expect("Could not locate Windows SDK rc.exe to embed application icon");
        let res_file = Path::new(&out_dir).join("app_icon.res");
        let status = Command::new(&rc_exe)
            .args(["/fo", res_file.to_str().unwrap(), "assets/app.rc"])
            .status()
            .expect("Failed to execute rc.exe");
        assert!(status.success(), "rc.exe failed with status {status}");
        println!("cargo:rustc-link-arg={}", res_file.display());
    } else {
        let res_obj = Path::new(&out_dir).join("app_icon.o");
        let status = Command::new("windres")
            .args([
                "assets/app.rc",
                "-O",
                "coff",
                "-o",
                res_obj.to_str().unwrap(),
            ])
            .status()
            .expect("Failed to execute windres");
        assert!(status.success(), "windres failed with status {status}");
        println!("cargo:rustc-link-arg={}", res_obj.display());
    }
}
