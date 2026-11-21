use std::{env, fs, path::Path};

fn add_sources(build: &mut cc::Build, directory: &Path) {
    let mut entries: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            add_sources(build, &path);
        } else if path.extension().is_some_and(|ext| ext == "cpp") {
            build.file(path);
        }
    }
}

fn main() {
    assert_eq!(
        env::var("CARGO_CFG_TARGET_OS").unwrap(),
        "windows",
        "bit7z currently supports Windows MSVC only"
    );
    assert_eq!(
        env::var("CARGO_CFG_TARGET_ENV").unwrap(),
        "msvc",
        "bit7z currently supports Windows MSVC only"
    );
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++14")
        .warnings(false)
        .flag("/EHsc")
        .flag("/Zc:__cplusplus")
        .flag("/utf-8")
        .define("UNICODE", None)
        .define("_UNICODE", None)
        .define("WIN32_LEAN_AND_MEAN", None)
        .define("NOMINMAX", None)
        .define("BIT7Z_AUTO_FORMAT", None)
        .define("BIT7Z_DETECT_FROM_EXTENSION", None)
        .define("BIT7Z_USE_NATIVE_STRING", None)
        .define("BIT7Z_DISABLE_USE_STD_FILESYSTEM", None)
        .include("vendor/bit7z/include")
        .include("vendor/bit7z/include/bit7z")
        .include("vendor/bit7z/src")
        .include("vendor/7zip/CPP")
        .include("vendor/filesystem/include")
        .file("native/bridge.cpp");
    // cc chooses the target compiler and CRT according to Cargo, including cross builds.
    add_sources(&mut build, Path::new("vendor/bit7z/src"));
    build.compile("driverindexer_bit7z");
    println!("cargo:rustc-link-lib=oleaut32");
    println!("cargo:rustc-link-lib=user32");
    for path in ["native", "vendor/bit7z", "vendor/7zip", "vendor/filesystem"] {
        println!("cargo:rerun-if-changed={path}");
    }
}
