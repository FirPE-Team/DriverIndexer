//! Opt-in CLI integration: the product manifest supplies the supported Windows
//! versions needed by drvstore. The Rust test runner has no product manifest.
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
fn names(root: &Path) -> BTreeSet<String> {
    fs::read_dir(root.join("Windows/INF"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| DriverIndexer::driver_management::is_oem(n))
        .collect()
}
fn command(root: &Path, verb: &str, args: &[&std::ffi::OsStr]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_DriverIndexer"))
        .args(["--language", "en", verb])
        .arg(root)
        .args(args)
        .output()
        .unwrap()
}
fn success(output: std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        output.status.success(),
        "{}\n{}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );
    stdout
}
#[test]
#[ignore = "writes only to D:/Project/FirPE/Mount; requires explicit environment opt-in"]
fn disposable_image_round_trip() {
    assert_eq!(
        std::env::var("DRIVERINDEXER_OFFLINE_WRITE_TEST").as_deref(),
        Ok("1")
    );
    let root = PathBuf::from(
        std::env::var_os("DRIVERINDEXER_OFFLINE_TEST_ROOT").expect("set disposable root"),
    );
    assert_eq!(
        root.canonicalize().unwrap(),
        Path::new("D:/Project/FirPE/Mount").canonicalize().unwrap()
    );
    assert!(
        DriverIndexer::driver_management::DriverTarget::discover(&root)
            .unwrap()
            .offline
    );
    let source =
        PathBuf::from(std::env::var_os("DRIVERINDEXER_TEST_INF").expect("set extracted test INF"));
    let original = names(&root);
    success(command(&root, "import", &[source.as_os_str()]));
    let added: Vec<_> = names(&root).difference(&original).cloned().collect();
    struct Cleanup {
        root: PathBuf,
        names: Vec<String>,
    }
    impl Drop for Cleanup {
        fn drop(&mut self) {
            for name in &self.names {
                let _ = command(&self.root, "remove", &["--inf".as_ref(), name.as_ref()]);
            }
        }
    }
    let mut cleanup = Cleanup {
        root: root.clone(),
        names: added.clone(),
    };
    assert_eq!(
        added.len(),
        1,
        "fixture already exists or failed to publish; preexisting packages will not be removed"
    );
    success(command(&root, "import", &[source.as_os_str()]));
    assert_eq!(names(&root).len(), original.len() + 1);
    let text = success(command(&root, "list", &[]));
    let record = text
        .split("Driver Info:")
        .find(|block| {
            block
                .lines()
                .any(|line| line.trim() == format!("OEM Name: {}", added[0]))
        })
        .expect("newly published package must be listed");
    let stored = PathBuf::from(
        record
            .lines()
            .find_map(|l| l.strip_prefix("Inf Path: "))
            .unwrap(),
    );
    let output =
        DriverIndexer::temp_workspace::TempWorkspace::create("offline-export-test").unwrap();
    success(command(
        &root,
        "export",
        &[
            output.path().as_os_str(),
            "--inf".as_ref(),
            added[0].as_ref(),
        ],
    ));
    // Original name and published name select exactly the same package.
    success(command(
        &root,
        "export",
        &[
            output.path().as_os_str(),
            "--inf".as_ref(),
            stored.file_name().unwrap(),
        ],
    ));
    let exported = walkdir::WalkDir::new(output.path())
        .into_iter()
        .map(Result::unwrap)
        .find(|e| e.file_type().is_file() && e.file_name() == stored.file_name().unwrap())
        .unwrap()
        .into_path();
    for entry in walkdir::WalkDir::new(stored.parent().unwrap()) {
        let entry = entry.unwrap();
        if entry.file_type().is_file() {
            let rel = entry.path().strip_prefix(stored.parent().unwrap()).unwrap();
            assert_eq!(
                fs::read(entry.path()).unwrap(),
                fs::read(exported.parent().unwrap().join(rel)).unwrap()
            );
        }
    }
    success(command(
        &root,
        "remove",
        &["--inf".as_ref(), added[0].as_ref()],
    ));
    cleanup.names.clear();
    assert_eq!(names(&root), original);
}
