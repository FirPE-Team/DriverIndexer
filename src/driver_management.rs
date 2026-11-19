//! Shared target, records and result types for driver-store management.
use crate::utils::{
    driver_store::{DriverPackageVersionInfo, DriverStore},
    utils::{
        get_current_system_root, get_file_version, get_offline_system_arch, is_offline_system,
    },
};
use anyhow::{Context, Result, anyhow};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct DriverTarget {
    pub root: PathBuf,
    pub windows: PathBuf,
    pub offline: bool,
    pub architecture: u16,
    pub version: (u16, u16, u16, u16),
    pub library: PathBuf,
}
impl DriverTarget {
    pub fn discover(root: &Path) -> Result<Self> {
        let resolved = root.canonicalize().context("resolve target image")?;
        // drvstore rejects verbatim DOS paths even when the filesystem accepts them.
        let root = PathBuf::from(
            resolved
                .to_string_lossy()
                .strip_prefix(r"\\?\")
                .unwrap_or(&resolved.to_string_lossy()),
        );
        let windows = root.join("Windows");
        let machine = get_offline_system_arch(&root)?;
        let architecture = match machine {
            0x14c => 0,
            0x8664 => 9,
            0xaa64 => 12,
            _ => return Err(anyhow!("unsupported image architecture: {machine:#x}")),
        };
        let offline = is_offline_system(&root)?;
        let version = get_file_version(&windows.join("System32/cmd.exe"))
            .ok_or_else(|| anyhow!("read target Windows version"))?;
        let host = get_current_system_root()?;
        let host_version = get_file_version(&host.join("System32/cmd.exe"))
            .ok_or_else(|| anyhow!("read host Windows version"))?;
        if offline && (host_version.0, host_version.1) < (6, 2) && (version.0, version.1) >= (6, 2)
        {
            return Err(anyhow!(
                "servicing a Win8+ image from a pre-Win8 host is unsupported"
            ));
        }
        let library = if offline && (version.0, version.1) < (6, 2) {
            let directory = match (std::env::consts::ARCH, machine) {
                ("x86", 0x14c) | ("x86_64", 0x8664) | ("aarch64", 0xaa64) => "System32",
                ("x86", 0x8664) | ("aarch64", 0x14c) => "SysWOW64",
                _ => {
                    return Err(anyhow!(
                        "unsupported legacy image/process architecture combination"
                    ));
                }
            };
            windows.join(directory).join("drvstore.dll")
        } else {
            host.join("System32/drvstore.dll")
        };
        Ok(Self {
            root,
            windows,
            offline,
            architecture,
            version,
            library,
        })
    }
    pub fn api(&self) -> Result<DriverStore> {
        Ok(DriverStore::new(Some(&self.library))?)
    }
}

#[derive(Debug, Clone)]
pub struct DriverRecord {
    pub published_name: String,
    pub inf_path: PathBuf,
    pub info: DriverPackageVersionInfo,
    pub class_description: String,
    pub critical: bool,
    pub inbox: bool,
    pub size_bytes: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemStatus {
    Success,
    NotApplicable,
    Failed,
}
#[derive(Debug, Clone)]
pub struct ItemResult {
    pub path: PathBuf,
    pub status: ItemStatus,
    pub message: String,
    pub code: Option<u32>,
}
#[derive(Debug, Default)]
pub struct BatchResult {
    pub items: Vec<ItemResult>,
}
impl BatchResult {
    pub fn counts(&self) -> (usize, usize, usize, usize) {
        let count = |s| self.items.iter().filter(|i| i.status == s).count();
        (
            count(ItemStatus::Success),
            count(ItemStatus::Failed),
            count(ItemStatus::NotApplicable),
            self.items.len(),
        )
    }
    pub fn ensure_success(&self) -> Result<()> {
        let (_, failed, _, _) = self.counts();
        if failed == 0 {
            Ok(())
        } else {
            Err(anyhow!("{failed} driver operation(s) failed"))
        }
    }
}

/// Only operations needed by a workflow are called; fake implementations can
/// assert the same ordering without changing a real machine or registry.
pub trait DriverBackend {
    fn find(&self, inf: &Path) -> Result<Option<PathBuf>>;
    fn info(&self, inf: &Path) -> Result<DriverPackageVersionInfo>;
    fn applicable(&self, inf: &Path) -> Result<bool>;
    fn import(&self, inf: &Path) -> Result<PathBuf>;
    fn critical(&self, inf: &Path, remove: bool) -> Result<()>;
    fn remove_devices(&self, published_name: &str, published_path: &Path) -> Result<()>;
    fn delete(&self, inf: &Path) -> Result<()>;
    fn rescan(&self) -> Result<()>;
}

/// The reference classifies critical packages by this exact class GUID table.
const CRITICAL_CLASSES: [u128; 17] = [
    0x4d36e97de32511cebfc108002be10318,
    0x4d36e966e32511cebfc108002be10318,
    0x50127dc30f36415ea6cc4cb3be910b65,
    0x4d36e96be32511cebfc108002be10318,
    0x4d36e96fe32511cebfc108002be10318,
    0x745a17a074d311d0b6fe00a0c90f57da,
    0x4d36e978e32511cebfc108002be10318,
    0x4d36e96ae32511cebfc108002be10318,
    0x4d36e97be32511cebfc108002be10318,
    0x4d36e967e32511cebfc108002be10318,
    0x4d36e965e32511cebfc108002be10318,
    0x4d36e980e32511cebfc108002be10318,
    0x4d36e969e32511cebfc108002be10318,
    0x36fc9e60c46511cf8056444553540000,
    0x6bdd1fc1810f11d0bec708002be2092f,
    0xc459df55db0811d1b00900a0c9081ff6,
    0x71a27cdd812a11d0bec708002be2092f,
];
pub fn is_critical(guid: &windows::core::GUID) -> bool {
    CRITICAL_CLASSES
        .iter()
        .any(|n| windows::core::GUID::from_u128(*n) == *guid)
}
pub fn is_oem(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    // Same prefix classification as the reference; callers only supply INF basenames.
    name.starts_with("oem") && name.ends_with(".inf") && !name.contains(['/', '\\', ':'])
}
#[derive(Debug, Default, Clone, Copy)]
pub struct DriverFilter<'a> {
    pub include_system: bool,
    pub system_only: bool,
    pub inf: Option<&'a str>,
    pub class: Option<&'a [String]>,
    pub exclude_class: Option<&'a [String]>,
    pub provider: Option<&'a [String]>,
}
impl DriverFilter<'_> {
    pub fn matches(&self, record: &DriverRecord) -> bool {
        let contains = |values: Option<&[String]>, value: &str| {
            values.is_none_or(|v| v.iter().any(|s| s.eq_ignore_ascii_case(value)))
        };
        (!self.system_only || record.inbox)
            && (self.include_system || self.system_only || !record.inbox)
            && self.inf.is_none_or(|inf| {
                record.published_name.eq_ignore_ascii_case(inf)
                    || record
                        .inf_path
                        .file_name()
                        .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(inf))
            })
            && contains(self.class, &record.info.class_name)
            && !self.exclude_class.is_some_and(|v| {
                v.iter()
                    .any(|c| c.eq_ignore_ascii_case(&record.info.class_name))
            })
            && contains(self.provider, &record.info.provider_name)
    }
}

pub struct NativeBackend<'a> {
    pub target: &'a DriverTarget,
    pub api: &'a DriverStore,
    pub store: crate::utils::driver_store::StoreSession<'a>,
}
impl<'a> NativeBackend<'a> {
    pub fn new(target: &'a DriverTarget, api: &'a DriverStore) -> Result<Self> {
        Ok(Self {
            target,
            api,
            store: api.session(&target.windows, target.offline)?,
        })
    }
}
impl DriverBackend for NativeBackend<'_> {
    fn find(&self, inf: &Path) -> Result<Option<PathBuf>> {
        Ok(self.store.find(inf, self.target.architecture)?)
    }
    fn info(&self, inf: &Path) -> Result<DriverPackageVersionInfo> {
        Ok(self.api.package(inf, self.target.architecture)?.info()?)
    }
    fn applicable(&self, inf: &Path) -> Result<bool> {
        Ok(self
            .api
            .package(inf, self.target.architecture)?
            .applicable()?)
    }
    fn import(&self, inf: &Path) -> Result<PathBuf> {
        Ok(self.store.import(inf, self.target.architecture)?)
    }
    fn critical(&self, inf: &Path, remove: bool) -> Result<()> {
        Ok(self.store.critical(inf, remove)?)
    }
    fn delete(&self, inf: &Path) -> Result<()> {
        Ok(self.store.delete(inf)?)
    }
    fn remove_devices(&self, _: &str, _: &Path) -> Result<()> {
        Err(anyhow!("online removal backend not enabled"))
    }
    fn rescan(&self) -> Result<()> {
        if self.target.offline || crate::utils::setupapi::SetupAPI::rescan() {
            Ok(())
        } else {
            Err(anyhow!("device rescan failed"))
        }
    }
}

pub fn published_infs(windows: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(windows.join("INF"))? {
        let entry = entry?;
        if entry.file_type()?.is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("inf"))
        {
            paths.push(entry.path());
        }
    }
    paths.sort_by_key(|p| p.to_string_lossy().to_ascii_lowercase());
    Ok(paths)
}
pub fn package_size(path: &Path) -> Result<u64> {
    let mut size = 0u64;
    for entry in walkdir::WalkDir::new(path).follow_links(false) {
        let entry = entry?;
        if entry.file_type().is_file() {
            size = size
                .checked_add(entry.metadata()?.len())
                .ok_or_else(|| anyhow!("package size overflow"))?;
        }
    }
    Ok(size)
}
pub fn enumerate(
    backend: &impl DriverBackend,
    windows: &Path,
    filter: DriverFilter<'_>,
) -> Result<Vec<DriverRecord>> {
    let mut records = Vec::new();
    for inf in published_infs(windows)? {
        let name = inf.file_name().unwrap().to_string_lossy().into_owned();
        let inbox = !is_oem(&name);
        if (filter.system_only && !inbox)
            || (!filter.include_system && !filter.system_only && inbox)
        {
            continue;
        }
        let read = || -> Result<Option<DriverRecord>> {
            let Some(path) = backend.find(&inf)? else {
                return Err(anyhow!("published INF has no repository package"));
            };
            let info = backend.info(&path)?;
            let class_description =
                crate::utils::setupapi::SetupAPI::get_class_description_from_guid(&info.class_guid)
                    .unwrap_or_default();
            let mut record = DriverRecord {
                published_name: name.clone(),
                inf_path: path,
                critical: is_critical(&info.class_guid),
                info,
                class_description,
                inbox,
                size_bytes: 0,
            };
            if !filter.matches(&record) {
                return Ok(None);
            }
            record.size_bytes = package_size(
                record
                    .inf_path
                    .parent()
                    .ok_or_else(|| anyhow!("invalid repository INF path"))?,
            )?;
            Ok(Some(record))
        };
        match read() {
            Ok(Some(record)) => records.push(record),
            Ok(None) => {}
            Err(e) => crate::utils::console::write_console(
                crate::utils::console::ConsoleType::Warning,
                &format!("{}: {e:#}", inf.display()),
            ),
        }
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record() -> DriverRecord {
        DriverRecord {
            published_name: "oem42.inf".into(),
            inf_path: PathBuf::from("repo/display.inf"),
            info: DriverPackageVersionInfo {
                architecture: 9,
                provider_name: "Vendor".into(),
                driver_date: Default::default(),
                driver_version: "1.2.3.4".into(),
                class_guid: windows::core::GUID::from_u128(0x4d36e968e32511cebfc108002be10318),
                class_name: "Display".into(),
                catalog_file: String::new(),
            },
            class_description: "Display adapters".into(),
            critical: false,
            inbox: false,
            size_bytes: 0,
        }
    }
    #[test]
    fn critical_table_and_oem_classification() {
        assert_eq!(CRITICAL_CLASSES.len(), 17);
        for n in CRITICAL_CLASSES {
            assert!(is_critical(&windows::core::GUID::from_u128(n)));
        }
        assert!(!is_critical(&record().info.class_guid));
        assert!(is_oem("OEM42.INF"));
        assert!(!is_oem("../oem42.inf"));
        assert!(!is_oem("disk.inf"));
    }
    #[test]
    fn filters_intersect_and_exclusion_wins() {
        let r = record();
        let class = vec!["display".into(), "Net".into()];
        let provider = vec!["vendor".into()];
        let f = DriverFilter {
            class: Some(&class),
            provider: Some(&provider),
            inf: Some("OEM42.INF"),
            ..Default::default()
        };
        assert!(f.matches(&r));
        assert!(
            DriverFilter {
                inf: Some("DISPLAY.inf"),
                ..f
            }
            .matches(&r)
        );
        assert!(
            !DriverFilter {
                exclude_class: Some(&class),
                ..f
            }
            .matches(&r)
        );
        assert!(
            !DriverFilter {
                system_only: true,
                ..f
            }
            .matches(&r)
        );
        let mut inbox = r.clone();
        inbox.inbox = true;
        assert!(!DriverFilter::default().matches(&inbox));
        assert!(
            DriverFilter {
                include_system: true,
                ..Default::default()
            }
            .matches(&inbox)
        );
    }
    #[test]
    fn current_layer_enumeration_and_recursive_size() {
        let temp = crate::temp_workspace::TempWorkspace::create("enumerate-test").unwrap();
        let inf = temp.path().join("INF");
        std::fs::create_dir_all(inf.join("nested")).unwrap();
        std::fs::write(inf.join("OEM1.INF"), b"123").unwrap();
        std::fs::write(inf.join("nested/hidden.inf"), b"12345").unwrap();
        assert_eq!(published_infs(temp.path()).unwrap().len(), 1);
        assert_eq!(package_size(&inf).unwrap(), 8);
    }
}

impl ItemResult {
    pub fn error(path: PathBuf, error: anyhow::Error) -> Self {
        let code = error
            .chain()
            .find_map(|e| {
                e.downcast_ref::<crate::utils::driver_store::DrvstoreError>()
                    .and_then(|e| e.code())
            })
            .or_else(|| {
                error.chain().find_map(|e| {
                    e.downcast_ref::<std::io::Error>()
                        .and_then(|e| e.raw_os_error())
                        .map(|n| n as u32)
                })
            });
        Self {
            path,
            status: ItemStatus::Failed,
            message: format!("{error:#}"),
            code,
        }
    }
}
pub fn import_infs(backend: &impl DriverBackend, paths: &[PathBuf], online: bool) -> BatchResult {
    let mut batch = BatchResult::default();
    for path in paths {
        let import = || -> Result<ItemResult> {
            if let Some(existing) = backend.find(path)? {
                return Ok(ItemResult {
                    path: path.clone(),
                    status: ItemStatus::Success,
                    message: format!("already imported: {}", existing.display()),
                    code: None,
                });
            }
            if !backend.applicable(path)? {
                return Ok(ItemResult {
                    path: path.clone(),
                    status: ItemStatus::NotApplicable,
                    message: String::new(),
                    code: Some(0x8007000a),
                });
            }
            let info = backend.info(path)?;
            let imported = backend.import(path)?;
            let mut warnings = Vec::new();
            if is_critical(&info.class_guid)
                && let Err(e) = backend.critical(&imported, false)
            {
                warnings.push(format!("critical reflection: {e:#}"));
            }
            if online && let Err(e) = backend.rescan() {
                warnings.push(format!("rescan: {e:#}"));
            }
            Ok(ItemResult {
                path: path.clone(),
                status: ItemStatus::Success,
                message: warnings.join("; "),
                code: None,
            })
        };
        batch
            .items
            .push(import().unwrap_or_else(|e| ItemResult::error(path.clone(), e)));
    }
    batch
}

#[cfg(test)]
mod workflow_tests {
    use super::*;
    use std::cell::RefCell;
    struct Fake {
        trace: RefCell<Vec<&'static str>>,
        existing: bool,
        applicable: bool,
        fail: Option<&'static str>,
        critical: bool,
    }
    impl Fake {
        fn new() -> Self {
            Self {
                trace: RefCell::new(Vec::new()),
                existing: false,
                applicable: true,
                fail: None,
                critical: true,
            }
        }
        fn step(&self, name: &'static str) -> Result<()> {
            self.trace.borrow_mut().push(name);
            if self.fail == Some(name) {
                Err(crate::utils::driver_store::DrvstoreError::Win32Error(5).into())
            } else {
                Ok(())
            }
        }
    }
    impl DriverBackend for Fake {
        fn find(&self, _: &Path) -> Result<Option<PathBuf>> {
            self.step("find")?;
            Ok(self.existing.then(|| PathBuf::from("repo/driver.inf")))
        }
        fn info(&self, _: &Path) -> Result<DriverPackageVersionInfo> {
            self.step("info")?;
            Ok(DriverPackageVersionInfo {
                architecture: 9,
                provider_name: "Vendor".into(),
                driver_date: Default::default(),
                driver_version: "1.2.3.4".into(),
                class_guid: windows::core::GUID::from_u128(if self.critical {
                    CRITICAL_CLASSES[0]
                } else {
                    0
                }),
                class_name: "System".into(),
                catalog_file: String::new(),
            })
        }
        fn applicable(&self, _: &Path) -> Result<bool> {
            self.step("applicable")?;
            Ok(self.applicable)
        }
        fn import(&self, _: &Path) -> Result<PathBuf> {
            self.step("import")?;
            Ok("repo/driver.inf".into())
        }
        fn critical(&self, _: &Path, remove: bool) -> Result<()> {
            self.step(if remove { "unreflect" } else { "reflect" })
        }
        fn remove_devices(&self, _: &str, _: &Path) -> Result<()> {
            self.step("devices")
        }
        fn delete(&self, _: &Path) -> Result<()> {
            self.step("delete")
        }
        fn rescan(&self) -> Result<()> {
            self.step("rescan")
        }
    }
    #[test]
    fn import_order_and_idempotence() {
        let f = Fake::new();
        let paths = vec!["driver.inf".into()];
        assert_eq!(import_infs(&f, &paths, true).counts(), (1, 0, 0, 1));
        assert_eq!(
            *f.trace.borrow(),
            ["find", "applicable", "info", "import", "reflect", "rescan"]
        );
        let f = Fake {
            existing: true,
            ..Fake::new()
        };
        assert!(import_infs(&f, &paths, false).ensure_success().is_ok());
        assert_eq!(*f.trace.borrow(), ["find"]);
    }
    #[test]
    fn not_applicable_and_errors_do_not_abort_batch() {
        let paths = vec!["a.inf".into(), "b.inf".into()];
        let f = Fake {
            applicable: false,
            ..Fake::new()
        };
        let b = import_infs(&f, &paths, false);
        assert_eq!(b.counts(), (0, 0, 2, 2));
        assert!(b.ensure_success().is_ok());
        for step in ["find", "applicable", "info", "import"] {
            let f = Fake {
                fail: Some(step),
                ..Fake::new()
            };
            let b = import_infs(&f, &paths, false);
            assert_eq!(b.counts(), (0, 2, 0, 2));
            assert!(b.ensure_success().is_err());
            assert_eq!(b.items[0].code, Some(5));
        }
        for step in ["reflect", "rescan"] {
            let f = Fake {
                fail: Some(step),
                ..Fake::new()
            };
            assert_eq!(import_infs(&f, &paths, true).counts(), (2, 0, 0, 2));
        }
    }
}
