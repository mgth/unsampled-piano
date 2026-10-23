//! Windows: embeds the application icon (assets/icon.ico) in the executable,
//! for Explorer, the taskbar and the installer's shortcuts.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/icon.ico");
        res.compile().expect("Windows resources (icon)");
    }
}
