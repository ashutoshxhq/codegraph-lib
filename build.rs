fn main() {
    println!("cargo:rustc-link-search=native=/opt/homebrew/lib");
    println!("cargo:rustc-link-lib=static=tree-sitter");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-arg=-mmacosx-version-min=15.3");
    }
}
