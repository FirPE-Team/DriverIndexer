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
        let root = root.canonicalize().context("resolve target image")?;
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
