//! Writes the generated files that come with this crate into `OUT_DIR`, naming this workspace's root where
//! they name source files.
use std::{env, fs, path::PathBuf};

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let root = manifest.join("../..");
    let root = root.to_str().expect("a UTF-8 path").replace('\\', "/");
    println!("cargo:rerun-if-changed=generated");
    for entry in fs::read_dir(manifest.join("generated")).expect("reading generated/") {
        let path = entry.expect("reading generated/").path();
        let code = fs::read_to_string(&path).expect("reading a generated file");
        let name = path.file_name().expect("a file name");
        fs::write(out.join(name), code.replace("{horizon-api-source-root}", &root)).expect("writing OUT_DIR");
    }
}
