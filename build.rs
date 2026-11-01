// Embeds the TXT Merge icon and name in the Windows exe.
fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "TXT Merge");
        res.set("FileDescription", "TXT Merge - Merger & Editor");
        res.compile().expect("could not embed the Windows icon");
    }
}
