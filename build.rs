fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("logo.ico");
        res.set("ProductName", "rust-nv");
        res.set("FileDescription", "Notational Velocity for Windows");
        res.compile().expect("Failed to compile Windows resources");
    }
}
