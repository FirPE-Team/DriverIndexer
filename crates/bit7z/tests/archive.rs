#![cfg(windows)]
use bit7z::{Bit7z, CompressionOptions, ExtractOptions, FileRange};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "bit7z-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}
fn library() -> Bit7z {
    let architecture = if cfg!(target_arch = "x86") {
        "assets-x86"
    } else if cfg!(target_arch = "aarch64") {
        "assets-ARM64"
    } else {
        "assets-x64"
    };
    let dll = std::env::var_os("BIT7Z_TEST_DLL")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(architecture)
                .join("7z.dll")
        });
    Bit7z::new(dll).unwrap()
}

#[test]
fn enumerates_and_extracts_selected_unicode_files() {
    let temp = Temp::new();
    let mut archive = library().open(fixture("sample.zip"), None).unwrap();
    let entries = archive.entries().unwrap();
    assert_eq!(entries.len(), 4);
    let entry = entries
        .iter()
        .find(|e| e.path == Path::new("中文 目录/driver.INF"))
        .unwrap();
    archive
        .extract(
            &[entry.index, entry.index],
            &temp.0,
            ExtractOptions::default(),
        )
        .unwrap();
    assert_eq!(
        fs::read(temp.0.join(&entry.path)).unwrap(),
        b"[Version]\nSignature=\"$Windows NT$\"\n"
    );
    assert!(!temp.0.join("readme.txt").exists());
    fs::write(temp.0.join(&entry.path), b"preserved").unwrap();
    archive
        .extract(&[entry.index], &temp.0, ExtractOptions::default())
        .unwrap();
    assert_eq!(fs::read(temp.0.join(&entry.path)).unwrap(), b"preserved");
}

#[test]
fn flattens_and_rejects_existing_output_when_requested() {
    let temp = Temp::new();
    let mut archive = library().open(fixture("sample.zip"), None).unwrap();
    let options = ExtractOptions {
        retain_directories: false,
        skip_existing: false,
    };
    archive.extract(&[0], &temp.0, options).unwrap();
    assert!(temp.0.join("driver.INF").is_file());
    assert!(archive.extract(&[0], &temp.0, options).is_err());
}

#[test]
fn empty_selection_does_not_mean_all() {
    let temp = Temp::new();
    let mut archive = library().open(fixture("sample.zip"), None).unwrap();
    archive
        .extract(&[], temp.0.join("empty"), ExtractOptions::default())
        .unwrap();
    assert!(!temp.0.join("empty").exists());
    assert!(
        archive
            .extract(&[999], &temp.0, ExtractOptions::default())
            .is_err()
    );
}

#[test]
fn reports_password_corruption_and_dll_failures() {
    let temp = Temp::new();
    let lib = library();
    assert!(lib.open(fixture("encrypted.7z"), Some("wrong")).is_err());
    assert!(lib.open(fixture("encrypted.7z"), None).is_err());
    let mut archive = lib.open(fixture("encrypted.7z"), Some("secret")).unwrap();
    archive
        .extract(&[0], &temp.0, ExtractOptions::default())
        .unwrap();
    assert_eq!(
        fs::read(temp.0.join("secret.txt")).unwrap(),
        b"secret payload"
    );
    fs::write(temp.0.join("invalid.dll"), b"not a DLL").unwrap();
    let error = Bit7z::new(temp.0.join("invalid.dll")).unwrap_err();
    assert!(!error.message.contains('\u{fffd}'), "{}", error.message);
    assert!(Bit7z::new(temp.0.join("missing.dll")).is_err());
    assert!(lib.open(temp.0.join("invalid.dll"), None).is_err());
    assert!(
        lib.open(fixture("sample.zip"), Some("nul\0password"))
            .is_err()
    );
}

#[test]
fn rejects_traversal_and_symlinks() {
    let temp = Temp::new();
    for name in ["traversal.zip", "symlink.zip"] {
        let mut archive = library().open(fixture(name), None).unwrap();
        assert!(
            archive
                .extract(&[0], temp.0.join("output"), ExtractOptions::default())
                .is_err(),
            "{name}"
        );
    }
    assert!(!temp.0.join("escape.txt").exists());
}

#[test]
fn rejects_existing_junction_outside_destination() {
    let temp = Temp::new();
    let output = temp.0.join("out");
    let outside = temp.0.join("outside");
    fs::create_dir_all(&output).unwrap();
    fs::create_dir_all(&outside).unwrap();
    // Directory symlinks require developer mode/admin; junctions do not.
    let status = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(output.join("中文 目录"))
        .arg(&outside)
        .output()
        .unwrap();
    assert!(status.status.success());
    let mut archive = library().open(fixture("sample.zip"), None).unwrap();
    assert!(
        archive
            .extract(&[0], &output, ExtractOptions::default())
            .is_err()
    );
    fs::remove_dir(output.join("中文 目录")).unwrap();
    assert!(fs::read_dir(outside).unwrap().next().is_none());
}

#[test]
fn reads_only_the_requested_file_region() {
    let temp = Temp::new();
    let bytes = fs::read(fixture("sample.zip")).unwrap();
    let prefix = vec![0u8; 8192];
    let mut wrapped = prefix.clone();
    wrapped.extend_from_slice(&bytes);
    wrapped.extend_from_slice(b"not part of archive");
    let path = temp.0.join("wrapped.exe");
    fs::write(&path, wrapped).unwrap();
    let lib = library();
    let mut archive = lib
        .open_range(
            &path,
            FileRange {
                offset: prefix.len() as u64,
                length: bytes.len() as u64,
            },
            None,
        )
        .unwrap();
    assert_eq!(archive.entries().unwrap().len(), 4);
    archive
        .extract(&[0, 1, 2, 3], temp.0.join("out"), ExtractOptions::default())
        .unwrap();
    assert_eq!(
        fs::read(temp.0.join("out/other/driver.sys")).unwrap(),
        b"driver bytes"
    );
    assert!(
        lib.open_range(
            &path,
            FileRange {
                offset: u64::MAX,
                length: 4
            },
            None
        )
        .is_err()
    );
    assert!(
        lib.open_range(
            &path,
            FileRange {
                offset: 0,
                length: u64::MAX
            },
            None
        )
        .is_err()
    );
    assert!(
        lib.open_range(
            &path,
            FileRange {
                offset: 0,
                length: 0
            },
            None
        )
        .is_err()
    );
}

#[test]
fn independent_readers_work_concurrently_and_release_files() {
    let temp = Temp::new();
    let path = temp.0.join("copy.zip");
    fs::copy(fixture("sample.zip"), &path).unwrap();
    let lib = library();
    std::thread::scope(|scope| {
        for worker in 0..8 {
            let lib = lib.clone();
            let path = &path;
            let output = temp.0.join(worker.to_string());
            scope.spawn(move || {
                let mut archive = lib.open(path, None).unwrap();
                archive
                    .extract(&[0, 1, 2, 3], output, ExtractOptions::default())
                    .unwrap();
            });
        }
    });
    fs::remove_file(path).unwrap();
}

#[test]
fn compresses_directory_contents_with_default_and_custom_settings() {
    let temp = Temp::new();
    let input = temp.0.join("中文 输入");
    fs::create_dir_all(input.join("nested")).unwrap();
    fs::write(input.join("root.inf"), b"root inf").unwrap();
    fs::write(input.join("nested/file.sys"), b"driver payload").unwrap();
    let lib = library();
    for (index, options) in [
        CompressionOptions::default(),
        CompressionOptions {
            level: 0,
            solid_block_size: 0,
            threads: 1,
        },
    ]
    .into_iter()
    .enumerate()
    {
        let output = temp.0.join(format!("output-{index}.7z"));
        lib.compress_directory(&input, &output, options).unwrap();
        let mut archive = lib.open(&output, None).unwrap();
        let entries = archive.entries().unwrap();
        assert!(entries.iter().any(|e| e.path == Path::new("root.inf")));
        assert!(!entries.iter().any(|e| e.path.starts_with("中文 输入")));
        let destination = temp.0.join(format!("out-{index}"));
        archive
            .extract(
                &entries.iter().map(|e| e.index).collect::<Vec<_>>(),
                &destination,
                ExtractOptions::default(),
            )
            .unwrap();
        assert_eq!(
            fs::read(destination.join("nested/file.sys")).unwrap(),
            b"driver payload"
        );
        drop(archive);
        assert!(lib.compress_directory(&input, &output, options).is_err());
    }
    assert!(
        lib.compress_directory(
            &input,
            input.join("recursive.7z"),
            CompressionOptions::default()
        )
        .is_err()
    );
    assert!(
        lib.compress_directory(
            &input,
            temp.0.join("invalid.7z"),
            CompressionOptions {
                level: 2,
                ..CompressionOptions::default()
            }
        )
        .is_err()
    );
}
