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
        let _ = root;
        let api = self.target.api()?;
        let backend = crate::driver_management::NativeBackend::new(&self.target, &api)?;
        let filter = crate::driver_management::DriverFilter {
            include_system: system,
            system_only: only,
            class,
            exclude_class: exclude,
            provider,
            inf: None,
        };
        for r in crate::driver_management::enumerate(&backend, &self.target.windows, filter)? {
            crate::utils::console::write_plain(&format!(
                "Driver Info:\nInf Path: {}\nOEM Name: {}\nClass Name: {}\nClass Desc: {}\nClass GUID: {{{:?}}}\nProvider: {}\nVersion: {}\nDate: {}\nSize: {}\nCritical: {}\nInbox: {}\n",
                r.inf_path.display(),
                r.published_name,
                r.info.class_name,
                r.class_description,
                r.info.class_guid,
                r.info.provider_name,
                r.info.driver_version,
                crate::utils::utils::filetime_to_string(&r.info.driver_date)?,
                r.size_bytes,
                r.critical,
                r.inbox
            ));
        }
        Ok(())
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
