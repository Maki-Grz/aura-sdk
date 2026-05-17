use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let qnn_base = env::var("QNN_SDK_ROOT")
        .expect("QNN_SDK_ROOT is not set. Point it to your QAIRT/QNN SDK installation.");

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-env-changed=QNN_SDK_ROOT");

    let lib_dir = format!("{}/lib/aarch64-windows-msvc", qnn_base);
    println!("cargo:rustc-link-search={}", lib_dir);
    println!("cargo:rustc-link-lib=Genie");

    let out_dir = env::var("OUT_DIR").unwrap();
    let target_dir = Path::new(&out_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let dlls = [
        "Genie.dll",
        "QnnHtp.dll",
        "QnnSystem.dll",
        "QnnHtpNetRunExtensions.dll",
        "QnnHtpPrepare.dll",
    ];
    for dll in &dlls {
        let src = Path::new(&lib_dir).join(dll);
        let dest = target_dir.join(dll);
        if src.exists() {
            let _ = fs::copy(&src, &dest);
        }
    }

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}/include/Genie", qnn_base))
        .clang_arg(format!("-I{}/include/QNN", qnn_base))
        .formatter(bindgen::Formatter::Rustfmt)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("failed to generate Genie bindings");

    let bindings_file = PathBuf::from(out_dir).join("genie_bindings.rs");
    bindings
        .write_to_file(&bindings_file)
        .expect("Couldn't write bindings!");
}
