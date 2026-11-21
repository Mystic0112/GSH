fn main() {
    println!("cargo:rustc-check-cfg=cfg(has_obscura)");
    println!("cargo:rerun-if-env-changed=GSH_OBSCURA");
    if std::env::var_os("GSH_OBSCURA").is_some_and(|value| !value.is_empty()) {
        println!("cargo:rustc-cfg=has_obscura");
    }
}
