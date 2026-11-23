use std::env;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    println!("cargo::rerun-if-changed=klip.exe.manifest");
    embed_resource::compile("klip.rc", embed_resource::NONE)
        .manifest_required()
        .expect("the application manifest must be embedded for per-monitor DPI awareness");
}
