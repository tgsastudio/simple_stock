// Build script to embed the Windows application icon (.ico) and version info into simple_stock.exe.
// Supports both MinGW/GNU (windres) and MSVC (rc.exe on GitHub Actions windows-latest).
use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=assets/app.rc");
    println!("cargo:rerun-if-changed=assets/icon.ico");

    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "windows" {
        return;
    }

    let out_dir = env::var("OUT_DIR").unwrap();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();

    if target_env == "msvc" {
        let res_file = Path::new(&out_dir).join("app_icon.res");
        let status = Command::new("rc.exe")
            .args(["/fo", res_file.to_str().unwrap(), "assets/app.rc"])
            .status();
        match status {
            Ok(s) if s.success() => {
                println!("cargo:rustc-link-arg={}", res_file.display());
            }
            Ok(s) => {
                println!("cargo:warning=rc.exe exited with status {s}, icon not embedded");
            }
            Err(e) => {
                println!("cargo:warning=rc.exe not found ({e}), icon not embedded");
            }
        }
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
            .status();
        match status {
            Ok(s) if s.success() => {
                println!("cargo:rustc-link-arg={}", res_obj.display());
            }
            Ok(s) => {
                println!("cargo:warning=windres exited with status {s}, icon not embedded");
            }
            Err(e) => {
                println!("cargo:warning=windres not found ({e}), icon not embedded");
            }
        }
    }
}
