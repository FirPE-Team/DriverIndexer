//! Archive-only checks. No tests install drivers or execute a bundled EXE.
use DriverIndexer::temp_workspace::TempWorkspace;
use DriverIndexer::utils::sevenzip::SevenZip;
use std::fs;
use std::path::Path;

fn fixture() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/bit7z/tests/fixtures/sample.zip")
}

#[test]
fn embedded_dll_adapter_selects_files_without_a_helper_executable() {
    assert!(DriverIndexer::Asset::get("7z.exe").is_none());
    let workspace = TempWorkspace::create("archive-adapter").unwrap();
    let zip = SevenZip::new_in(workspace.path()).unwrap();
    zip.is_driver_package(&fixture(), None).unwrap();
    let output = workspace.path().join("selected");
    zip.extract_files_from_paths(&fixture(), None, &["*.inf", "*.CAT"], &output)
        .unwrap();
    assert!(output.join("中文 目录/driver.INF").is_file());
    assert!(output.join("中文 目录/driver.cat").is_file());
    assert!(!output.join("other/driver.sys").exists());
    zip.extract_files_from_path(
        &fixture(),
        None,
        "missing.*",
        &workspace.path().join("none"),
    )
    .unwrap();
    assert!(!workspace.path().join("none").exists());
    zip.extract_files_from_path(
        &fixture(),
        None,
        "中文 目录",
        &workspace.path().join("directory"),
    )
    .unwrap();
    assert!(
        workspace
            .path()
            .join("directory/中文 目录/driver.cat")
            .is_file()
    );
    zip.extract_files(
        &fixture(),
        None,
        "中文 目录\\driver.INF",
        &workspace.path().join("flat"),
    )
    .unwrap();
    assert!(workspace.path().join("flat/driver.INF").is_file());
}

#[test]
fn index_selection_preserves_spaces_and_prefers_index() {
    let workspace = TempWorkspace::create("archive-index").unwrap();
    let zip = SevenZip::new_in(workspace.path()).unwrap();
    let input = workspace.path().join("source");
    fs::create_dir_all(input.join("含 空格")).unwrap();
    fs::write(input.join("fallback.json"), b"fallback").unwrap();
    fs::write(input.join("含 空格/driver index.index"), b"preferred").unwrap();
    let archive = workspace.path().join("index.7z");
    zip.create_archive(&input, &archive).unwrap();
    let output = workspace.path().join("out");
    zip.extract_index(&archive, &output).unwrap();
    assert_eq!(
        fs::read(output.join("driver index.index")).unwrap(),
        b"preferred"
    );
    assert!(!output.join("fallback.json").exists());
    fs::remove_file(input.join("含 空格/driver index.index")).unwrap();
    let archive = workspace.path().join("fallback.7z");
    zip.create_archive(&input, &archive).unwrap();
    zip.extract_index(&archive, &output).unwrap();
    assert_eq!(fs::read(output.join("fallback.json")).unwrap(), b"fallback");
}

#[test]
fn package_detection_requires_a_real_inf_file() {
    let workspace = TempWorkspace::create("archive-detection").unwrap();
    let zip = SevenZip::new_in(workspace.path()).unwrap();
    let input = workspace.path().join("source");
    fs::create_dir_all(input.join("directory.inf")).unwrap();
    fs::write(input.join("not.inf.txt"), b"not an INF").unwrap();
    let archive = workspace.path().join("no-driver.7z");
    zip.create_archive(&input, &archive).unwrap();
    assert!(zip.is_driver_package(&archive, None).is_err());
}

#[test]
fn embedded_footer_selects_the_archive_region_and_rejects_invalid_ranges() {
    let workspace = TempWorkspace::create("archive-footer").unwrap();
    let zip = SevenZip::new_in(workspace.path()).unwrap();
    let archive = fs::read(fixture()).unwrap();
    let mut executable = vec![0; 4096];
    executable[..2].copy_from_slice(b"MZ");
    executable.extend_from_slice(&archive);
    executable.extend_from_slice(&4096u64.to_le_bytes());
    executable.extend_from_slice(&(archive.len() as u64).to_le_bytes());
    executable.extend_from_slice(&[0; 64]);
    executable.extend_from_slice(b"DRV_PKG!");
    let path = workspace.path().join("bundled.exe");
    fs::write(&path, &executable).unwrap();
    zip.is_driver_package(&path, None).unwrap();
    zip.extract_files_from_paths(
        &path,
        None,
        &["*.inf", "*.cat"],
        &workspace.path().join("out"),
    )
    .unwrap();
    assert!(workspace.path().join("out/中文 目录/driver.INF").is_file());
    let footer = executable.len() - 88;
    executable[footer..footer + 8].copy_from_slice(&u64::MAX.to_le_bytes());
    fs::write(&path, &executable).unwrap();
    assert!(zip.is_driver_package(&path, None).is_err());
}

#[test]
fn owned_workspace_lives_until_the_last_clone_is_dropped() {
    let zip = SevenZip::new().unwrap();
    let cloned = zip.clone();
    drop(zip);
    cloned.is_driver_package(&fixture(), None).unwrap();
    // Both the DLL and all temporary handles must be released by drop.
    drop(cloned);
}
