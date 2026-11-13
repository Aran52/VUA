fn main() {
    println!("cargo:rerun-if-changed=../../apps/desktop/packaging/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let target = std::env::var("TARGET").expect("Cargo target");
    let icon = std::path::Path::new("../../apps/desktop/packaging/icon.ico")
        .canonicalize()
        .expect("VUA application icon");
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output"));
    let script = output.join("vua-icon.rc");
    let resource = output.join("vua-icon.res");
    let icon_text = icon.to_str().expect("VUA icon path");
    let icon_text = icon_text
        .strip_prefix(r"\\?\")
        .unwrap_or(icon_text)
        .replace('\\', "/");
    std::fs::write(&script, format!("1 ICON \"{icon_text}\"\n"))
        .expect("write VUA icon resource script");
    let compiler =
        cc::windows_registry::find_tool(&target, "rc.exe").expect("Windows SDK resource compiler");
    let status = compiler
        .to_command()
        .arg("/nologo")
        .arg("/fo")
        .arg(&resource)
        .arg(&script)
        .status()
        .expect("compile VUA icon resource");
    assert!(status.success(), "VUA icon resource compilation failed");
    println!(
        "cargo:rustc-link-arg-bin=vua-distribution={}",
        resource.display()
    );
}
