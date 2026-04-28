fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // FFmpeg's Media Foundation backend references COM interface IIDs from these import libs.
        println!("cargo:rustc-link-lib=mfuuid");
        println!("cargo:rustc-link-lib=strmiids");
    }

    tauri_build::build()
}
