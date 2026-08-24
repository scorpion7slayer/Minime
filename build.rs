fn main() {
    println!("cargo:rerun-if-env-changed=MINIME_APP_ID");
    println!("cargo:rerun-if-changed=assets/minime.ico");
    println!("cargo:rerun-if-changed=packaging/windows/minime.rc");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("packaging/windows/minime.rc", embed_resource::NONE)
            .manifest_optional()
            .expect("unable to embed the Minime Windows icon");
    }
}
