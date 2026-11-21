//! Opt-in regression checks against local driver archives. All output is confined
//! to temporary workspaces; bundled executables are inspected, never executed.
use DriverIndexer::driver_index::DriverIndex;
use DriverIndexer::temp_workspace::TempWorkspace;
use DriverIndexer::utils::sevenzip::SevenZip;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Deserialize)]
struct ExpectedFile {
    path: String,
    size: u64,
    sha256: String,
}

fn packages() -> PathBuf {
    std::env::var_os("DRIVERINDEXER_TEST_DRIVE")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("testDrive"))
}
fn expected(name: &str) -> Vec<ExpectedFile> {
    let source = match name {
        "VMwareVMCI" => include_str!("fixtures/driver_archives/VMwareVMCI.manifest.json"),
        "DisplayVmWare" => include_str!("fixtures/driver_archives/DisplayVmWare.manifest.json"),
        _ => unreachable!(),
    };
    serde_json::from_str(source.trim_start_matches('\u{feff}')).unwrap()
}
fn assert_files(root: &Path, expected: &[&ExpectedFile]) {
    let actual: BTreeMap<_, _> = walkdir::WalkDir::new(root)
        .into_iter()
        .map(|item| item.unwrap())
        .filter(|item| item.file_type().is_file())
        .map(|item| {
            let bytes = fs::read(item.path()).unwrap();
            (
                item.path()
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
                (
                    bytes.len() as u64,
                    Sha256::digest(&bytes)
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<String>(),
                ),
            )
        })
        .collect();
    let expected: BTreeMap<_, _> = expected
        .iter()
        .map(|item| (item.path.clone(), (item.size, item.sha256.clone())))
        .collect();
    assert_eq!(actual, expected);
}
fn cli(arguments: &[&std::ffi::OsStr]) {
    let output = Command::new(env!("CARGO_BIN_EXE_DriverIndexer"))
        .args(["--language", "en"])
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "requires the two testDrive archives; run explicitly with --ignored"]
fn real_driver_packages_match_cli_baselines() {
    let workspace = TempWorkspace::create("driver-archive-baseline").unwrap();
    let zip = SevenZip::new_in(workspace.path()).unwrap();
    for name in ["VMwareVMCI", "DisplayVmWare"] {
        let package = packages().join(format!("{name}.7z"));
        assert!(package.is_file(), "Missing {}", package.display());
        zip.is_driver_package(&package, None).unwrap();
        let expected = expected(name);
        let all = workspace.path().join(format!("{name}-all"));
        zip.extract_files_from_path(&package, None, "*", &all)
            .unwrap();
        assert_files(&all, &expected.iter().collect::<Vec<_>>());
        let selected = workspace.path().join(format!("{name}-selected"));
        zip.extract_files_from_paths(&package, None, &["*.inf", "*.cat"], &selected)
            .unwrap();
        let metadata: Vec<_> = expected
            .iter()
            .filter(|file| file.path.ends_with(".inf") || file.path.ends_with(".cat"))
            .collect();
        assert_eq!(metadata.len(), 2);
        assert_files(&selected, &metadata);
        let folder = expected[0].path.split('/').next().unwrap();
        let directory = workspace.path().join(format!("{name}-directory"));
        zip.extract_files_from_path(&package, None, &folder.replace('/', "\\"), &directory)
            .unwrap();
        assert_files(&directory, &expected.iter().collect::<Vec<_>>());
    }
}

#[test]
#[ignore = "requires the two testDrive archives; run explicitly with --ignored"]
fn real_driver_indexes_and_packaged_executables_round_trip() {
    let workspace = TempWorkspace::create("driver-archive-workflows").unwrap();
    let zip = SevenZip::new_in(workspace.path()).unwrap();
    for (name, inf_path, hardware_id) in [
        (
            "VMwareVMCI",
            "VMware VMCI Bus Device/oem2.inf",
            "PCI\\VEN_15AD&DEV_0740&SUBSYS_074015AD",
        ),
        (
            "DisplayVmWare",
            "2019711/vm3d.inf",
            "PCI\\VEN_15AD&DEV_0405&SUBSYS_040515AD&REV_00",
        ),
    ] {
        let package = packages().join(format!("{name}.7z"));
        for compressed in [false, true] {
            let index = workspace.path().join(format!("{name}-{compressed}.index"));
            let mut arguments = vec![
                std::ffi::OsStr::new("index"),
                package.as_os_str(),
                index.as_os_str(),
            ];
            if compressed {
                arguments.push(std::ffi::OsStr::new("--compress"));
            }
            cli(&arguments);
            let index = DriverIndex::from_path(&index).unwrap();
            assert_eq!(index.drivers.len(), 1);
            assert_eq!(index.drivers[0].path.replace('\\', "/"), inf_path);
            assert!(
                index.drivers[0]
                    .hardware
                    .iter()
                    .any(|entry| entry.hardware_id.eq_ignore_ascii_case(hardware_id))
            );
            assert_eq!(index.drivers[0].hardware.len(), 2);
        }
        // Archive -> executable; inspect through the adapter's footer handling.
        let bundled = workspace.path().join(format!("{name}.exe"));
        cli(&[
            std::ffi::OsStr::new("pack"),
            package.as_os_str(),
            bundled.as_os_str(),
        ]);
        zip.is_driver_package(&bundled, None).unwrap();
        let output = workspace.path().join(format!("{name}-bundled"));
        zip.extract_files_from_path(&bundled, None, "*", &output)
            .unwrap();
        assert_files(&output, &expected(name).iter().collect::<Vec<_>>());
    }
    // Directory -> index + compressed archive -> executable, preserving root structure.
    let input = workspace.path().join("directory input");
    zip.extract_files_from_path(&packages().join("VMwareVMCI.7z"), None, "*", &input)
        .unwrap();
    let bundled = workspace.path().join("directory-bundled.exe");
    cli(&[
        std::ffi::OsStr::new("pack"),
        input.as_os_str(),
        bundled.as_os_str(),
    ]);
    let output = workspace.path().join("directory-bundled-output");
    zip.extract_files_from_path(&bundled, None, "*", &output)
        .unwrap();
    let index_path = output.join("directory input.index");
    let index = DriverIndex::from_path(&index_path).unwrap();
    assert_eq!(index.drivers.len(), 1);
    fs::remove_file(index_path).unwrap();
    assert_files(&output, &expected("VMwareVMCI").iter().collect::<Vec<_>>());
}
