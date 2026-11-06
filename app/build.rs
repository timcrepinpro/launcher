// Icône Windows de l'exe (logo feu de camp, version pixel).
fn main() {
    println!("cargo:rerun-if-changed=icone-app.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("icone-app.ico");
        if let Err(e) = res.compile() {
            println!("cargo:warning=icône non intégrée : {e}");
        }
    }
}
