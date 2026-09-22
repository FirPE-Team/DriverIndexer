//! Explicit Windows integration checks.
//!
//! These tests are ignored by default because they enumerate the local device
//! tree and may require an elevated process. Run them deliberately with:
//! `cargo test --test windows_manual -- --ignored`.

use std::env;

#[test]
#[ignore = "requires an administrator-capable Windows environment and real devices"]
fn enumerate_devices_from_setupapi() {
    let missing_only = env::var("DRIVERINDEXER_MISSING_ONLY")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let devices = DriverIndexer::hardware::enumerate_hardware(None, missing_only)
        .expect("SetupAPI device enumeration failed");
    println!(
        "enumerated {} device(s), missing_only={missing_only}",
        devices.len()
    );
}

#[test]
#[ignore = "requires a catalog path and WinVerifyTrust-capable Windows environment"]
fn verify_catalog_signature_from_environment() {
    let path = env::var_os("DRIVERINDEXER_CATALOG")
        .expect("set DRIVERINDEXER_CATALOG to a catalog or signed file path");
    let path = std::path::PathBuf::from(path);
    assert!(
        DriverIndexer::utils::utils::check_catalog_signature(&path),
        "catalog signature verification failed for {}",
        path.display()
    );
}
