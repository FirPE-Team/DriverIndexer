# bit7z Rust bridge

Windows MSVC crate wrapping vendored bit7z v4.1.0 through a private C ABI.
It requires a caller-provided, matching-architecture `7z.dll`. It does not embed
application assets, create application workspaces, start `7z.exe`, or download
native sources at build time. See [vendor/README.md](vendor/README.md) for exact
source revisions and licenses.

## Use

```rust,no_run
use bit7z::{Bit7z, CompressionOptions, ExtractOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let library = Bit7z::new(r"C:\tools\7z.dll")?;
    let mut archive = library.open(r"C:\drivers\package.7z", None)?;
    let selected: Vec<_> = archive.entries()?.into_iter()
        .filter(|entry| !entry.is_directory && entry.path.extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("inf")))
        .map(|entry| entry.index)
        .collect();
    archive.extract(&selected, r"C:\output", ExtractOptions::default())?;
    drop(archive);
    library.compress_directory(r"C:\input", r"C:\output.7z", CompressionOptions::default())?;
    Ok(())
}
```

`open_range(path, FileRange { offset, length }, password)` exposes a bounded,
seekable file view for archives inside larger files. It does not buffer the
whole archive in memory. The application is responsible for interpreting its
own executable footer and supplying a valid range.

`Archive` owns its reader, optional input stream, and loaded DLL; it is neither
`Send` nor `Sync`. Clone `Bit7z` and open an independent archive inside each
worker. `Bit7z` stores only an absolute DLL path and does not keep the DLL loaded.
Native calls are synchronous and copy all metadata before returning to callers.

Extraction sorts/deduplicates indices. Empty indices mean no extraction.
Defaults preserve directories and skip existing files; disabling `skip_existing`
reports a conflict instead of overwriting. Absolute paths, parent traversal,
alternate data stream paths, symbolic links, and existing destination links
escaping the extraction root are rejected. Callers must prevent concurrent
replacement of destination directories during extraction.

Compression creates 7z archives containing the input directory's contents.
Defaults are level 9, 8 MiB solid blocks, and automatic thread count. Supported
levels are 0/1/3/5/7/9; zero solid block size disables solid mode. Existing output
archives and output paths inside the input directory are rejected.

## Build and test

Install a Rust toolchain supporting edition 2024, Visual Studio C++ Build Tools,
and the Windows SDK. Cross compilation also requires the target C++ toolset and
Rust standard library. Cargo's `cc` dependency selects the target MSVC compiler
and CRT; no CMake installation is required.

```powershell
cargo test -p bit7z
cargo test -p bit7z --target i686-pc-windows-msvc
cargo build -p bit7z --release --target aarch64-pc-windows-msvc
```

Integration tests use the repository's architecture-specific `assets-*/7z.dll`.
When testing this crate separately, set `BIT7Z_TEST_DLL` to an absolute DLL path.
The small ZIP fixtures contain synthetic data; `encrypted.7z` contains only
`secret.txt` with the bytes `secret payload`, encrypted with password `secret`
and encrypted headers, created using 7-Zip 26.00. The traversal and symbolic-link
ZIP fixtures intentionally exercise path rejection.
