//! Driver management facade. Operations migrate individually from the legacy facade.
use crate::driver_management::DriverTarget;
use anyhow::Result;
use std::path::Path;
pub struct DriverManger {
    pub target: DriverTarget,
}
impl DriverManger {
    pub fn new(system_drive: &Path) -> Result<Self> {
        Ok(Self {
            target: DriverTarget::discover(system_drive)?,
        })
    }
    fn legacy(&self) -> Result<crate::legacy_driver_manager::DriverManger> {
        crate::legacy_driver_manager::DriverManger::new(&self.target.root)
    }
    pub fn list_driver(
        &self,
        root: &Path,
        system: bool,
        only: bool,
        class: Option<&[String]>,
        exclude: Option<&[String]>,
        provider: Option<&[String]>,
    ) -> Result<()> {
        self.legacy()?
            .list_driver(root, system, only, class, exclude, provider)
    }
    pub fn import_driver(
        &self,
        root: &Path,
        source: &Path,
        password: Option<&str>,
        matched: bool,
    ) -> Result<(u32, u32, u32)> {
        self.legacy()?
            .import_driver(root, source, password, matched)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn export_driver(
        &self,
        root: &Path,
        output: &Path,
        system: bool,
        only: bool,
        inf: Option<&str>,
        class: Option<&[String]>,
        exclude: Option<&[String]>,
        provider: Option<&[String]>,
    ) -> Result<(u32, u32, u32)> {
        self.legacy()?
            .export_driver(root, output, system, only, inf, class, exclude, provider)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn remove_driver(
        &self,
        root: &Path,
        system: bool,
        only: bool,
        inf: Option<&str>,
        class: Option<&[String]>,
        provider: Option<&[String]>,
        all: bool,
    ) -> Result<(u32, u32, u32)> {
        self.legacy()?
            .remove_driver(root, system, only, inf, class, provider, all)
    }
}
