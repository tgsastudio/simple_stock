// Build script to embed the Windows application icon (.ico) into simple_stock.exe
use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=assets/app.rc");
    println!("cargo:rerun-if-changed=assets/icon.ico");

    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let out_dir = env::var("OUT_DIR").unwrap();
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
