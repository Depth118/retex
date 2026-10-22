fn main() {
    // Embed the icon and version info so Explorer, the taskbar and shortcuts
    // show Retex instead of a generic executable.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=../../assets/icon.ico");
        winresource::WindowsResource::new()
            .set_icon("../../assets/icon.ico")
            .set("ProductName", "Retex")
            .set("FileDescription", "Retex")
            .compile()
            .expect("failed to embed Windows resources");
    }
}
