//! Read-only SetupAPI checks using synthetic INFs; no driver installation required.

use DriverIndexer::driver_index::{DriverArch, InfInfo};
use DriverIndexer::temp_workspace::TempWorkspace;
use DriverIndexer::utils::setupapi::SetupAPI;
use std::fs;

fn fixture() -> TempWorkspace {
    let workspace = TempWorkspace::create("inf-parsing-test").unwrap();
    let text = format!(
        r#"[Version]
Signature="$Windows NT$"
Class=System
ClassGuid={{4d36e97d-e325-11ce-bfc1-08002be10318}}
DriverVer=11/18/2026,1.2.3.4
CatalogFile=missing.cat

[Manufacturer]
%Vendor%=Models,NTamd64.10.0

[Models.NTamd64.10.0]
%Device%=Install,PCI\VEN_1234&DEV_5678,PCI\VEN_1234

[Install.NTamd64]
FeatureScore=0x80

[Strings]
Vendor="Test vendor"
Device="Test device"
{}="Long key value"
"#,
        "LongKey".repeat(32)
    );
    let bytes: Vec<u8> = std::iter::once(0xfeffu16)
        .chain(text.encode_utf16())
        .flat_map(u16::to_le_bytes)
        .collect();
    fs::write(workspace.path().join("test.inf"), bytes).unwrap();
    workspace
}

#[test]
fn finds_named_keys_and_first_line() {
    let workspace = fixture();
    let handle = SetupAPI::open_inf_file(&workspace.path().join("test.inf")).unwrap();

    for (section, key, expected) in [
        ("Version", "Class".to_string(), "System"),
        ("Version", "DriverVer".to_string(), "11/18/2026"),
        ("Install.NTamd64", "FeatureScore".to_string(), "0x80"),
        ("Strings", "LongKey".repeat(32), "Long key value"),
    ] {
        let context = SetupAPI::find_first_line(&handle, section, Some(&key)).unwrap();
        assert_eq!(SetupAPI::get_string_field(&context, 1).unwrap(), expected);
    }

    let first = SetupAPI::find_first_line(&handle, "Manufacturer", None).unwrap();
    assert_eq!(SetupAPI::get_string_field(&first, 1).unwrap(), "Models");
    assert!(SetupAPI::find_first_line(&handle, "Version", Some("MissingKey")).is_err());
    assert!(SetupAPI::find_first_line(&handle, "MissingSection", None).is_err());
}

#[test]
fn parallel_inf_parsing_preserves_required_and_optional_fields() {
    let workspace = fixture();
    let base = workspace.path();
    let inf = base.join("test.inf");
    let barrier = std::sync::Barrier::new(16);

    std::thread::scope(|scope| {
        for _ in 0..16 {
            let inf = &inf;
            let barrier = &barrier;
            scope.spawn(move || {
                barrier.wait();
                for _ in 0..256 {
                    let info = InfInfo::parse_inf(base, inf).unwrap();
                    assert_eq!(info.path, "test.inf");
                    assert_eq!(info.class, "System");
                    assert_eq!(info.date, "2026-11-18");
                    assert_eq!(info.version, "1.2.3.4");
                    assert_eq!(info.signature, 0xff);
                    assert_eq!(info.hardware.len(), 1);
                    let hardware = &info.hardware[0];
                    assert_eq!(hardware.desc, "Test device");
                    assert_eq!(hardware.arch, DriverArch::NTamd64);
                    assert_eq!(hardware.min_os_version, "10.0");
                    assert_eq!(hardware.hardware_id, r"PCI\VEN_1234&DEV_5678");
                    assert_eq!(hardware.compatible_ids, [r"PCI\VEN_1234"]);
                    assert_eq!(hardware.feature_score, 0x80);
                }
            });
        }
    });
}
