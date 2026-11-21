# bit7z migration verification

Validated on 2026-11-21 on Windows 10 Pro 22H2 (10.0.19045), x64,
with Rust/Cargo 1.98.1 and Visual Studio 2022 MSVC 14.38.33130.
x86 executables ran through WOW64. No drivers were installed, and generated
self-extracting executables were inspected without executing them.

## Results

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Passed |
| `cargo test -p bit7z --offline --target-dir target/bit7z-offline-validation` | Fresh native build and all 9 crate tests passed |
| `cargo test --workspace` | 53 passed; 4 explicitly opt-in tests ignored |
| `cargo test --workspace --target i686-pc-windows-msvc` | 53 passed; 4 explicitly opt-in tests ignored |
| `cargo test --test driver_archives -- --ignored` | Both real-package tests passed |
| `cargo test --target i686-pc-windows-msvc --test driver_archives -- --ignored` | Both real-package tests passed |
| `cargo build --release` | Passed; x64 PE machine `0x8664` |
| `cargo build --release --target i686-pc-windows-msvc` | Passed; x86 PE machine `0x014c` |
| `cargo build --release --target aarch64-pc-windows-msvc` | Passed; ARM64 PE machine `0xaa64` |

The four opt-in tests consist of two local driver-archive tests, which were
subsequently executed explicitly, and two existing real-device/signature tests,
which remain unexecuted. Both executable and bundled DLL machine types were
checked for all three architectures.

## Archive and workflow coverage

Synthetic fixtures exercise enumeration, Unicode paths, retained/flattened
directories, skip-existing behavior, empty selection, invalid indices, encrypted
headers, missing/wrong passwords, corrupt input, missing/invalid DLLs, NUL input,
parent traversal, symbolic links, existing escaping junctions, bounded file
regions, concurrent independent readers, file-handle release, compression root
structure, configurable compression, and existing-output errors.

Application adapter tests cover DLL-only embedded resources, recursive INF/CAT
selection, literal directories, no matches, index-over-JSON precedence, index
names containing spaces, true INF-file detection, footer range validation, and
cloned temporary workspace ownership.

The real-package tests compare full extraction and selected-directory extraction
against the old 7-Zip 26.00 CLI's exact path/size/SHA-256 manifests:

| Package | Files | Uncompressed bytes | Index result |
| --- | ---: | ---: | --- |
| VMwareVMCI.7z | 4 | 118,867 | 1 INF, 2 hardware entries |
| DisplayVmWare.7z | 35 | 60,644,245 | 1 INF, 2 hardware entries |

Both packages also pass recursive INF/CAT extraction (two files each), plain and
Zstandard-compressed index round trips, known INF-path/hardware-ID assertions,
archive-to-EXE packaging and extraction. VMCI additionally covers directory-to-EXE
packaging with an automatically generated index. Tests never write into testDrive.

The final x64 and x86 release binaries were separately run for indexing, reading
indices, and directory packaging. Each packing input contained the VMCI files and
two additional 5 MiB files. The payload was sliced using the actual 88-byte footer
and independently checked with the saved pre-migration CLI:

- CRC test succeeded.
- Every extracted file's SHA-256 matched the packing input.
- The two 5 MiB files occupied different solid blocks, confirming the configured
  8 MiB block limit instead of unlimited solid compression.

Original CLI copies, detailed listings, manifests, release smoke outputs, and
PE-architecture inspection results reside locally under `target/bit7z-validation`.
Small golden manifests are committed under `tests/fixtures/driver_archives`;
the original driver archives remain untracked.

## Compatibility limits

The main-branch build fix prevents ARM64 from linking an x64 XP thunk object
(the original failure was LNK1112). x86/x64 release builds retain the existing
VC-LTL/YY-Thunks configuration. The native bridge uses C++14, UTF-16 Windows
strings and vendored ghc::filesystem rather than std::filesystem.

Windows XP, Windows 7, and native ARM64 runtime behavior have not been tested.
Passing cross builds does not establish runtime compatibility on those systems.
The current integration targets Windows MSVC; callers must supply the correct
architecture's 7z.dll. Native source versions and licenses are recorded in
`crates/bit7z/vendor/README.md`.
