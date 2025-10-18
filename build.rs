fn main() {
    if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
        let mut res = winres::WindowsResource::new();
        let version = env!("CARGO_PKG_VERSION");
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "Taskbar Hide");
        res.set("FileDescription", "Taskbar Hide Utility");
        res.set("CompanyName", "AmN");
        res.set("LegalCopyright", "Copyright © 2025 AmN");
        res.set("OriginalFilename", "thide.exe");
        res.set("InternalName", "thide");
        res.set("ProductVersion", version);
        res.set("FileVersion", version);
        res.set(
            "Comments",
            "A lightweight utility to hide/show Windows taskbar | github.com/amnweb/thide",
        );
        res.set("LegalTrademarks", "");

        res.compile().unwrap_or_else(|_| {
            eprintln!("Warning: Could not embed resources in executable");
        });
    }
}
