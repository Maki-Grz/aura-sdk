use std::env;
use std::path::PathBuf;

fn main() {
    let qnn_base = env::var("QNN_SDK_ROOT")
        .expect("QNN_SDK_ROOT is not set. Point it to your QAIRT/QNN SDK installation.");

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-env-changed=QNN_SDK_ROOT");
    println!(
        "cargo:rustc-link-search={}/lib/aarch64-windows-msvc",
        qnn_base
    );
    println!("cargo:rustc-link-lib=Genie");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}/include/Genie", qnn_base))
        .clang_arg(format!("-I{}/include/QNN", qnn_base))
        .formatter(bindgen::Formatter::Rustfmt)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("failed to generate Genie bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let bindings_file = out_path.join("genie_bindings.rs");

    println!("cargo:warning=Generating bindings at {:?}", bindings_file);

    bindings
        .write_to_file(&bindings_file)
        .expect("Couldn't write bindings!");
}
