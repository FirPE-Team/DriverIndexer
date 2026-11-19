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
        source: &Path,
        password: Option<&str>,
        matched: bool,
    ) -> Result<crate::driver_management::BatchResult> {
        use crate::{
            driver_management::*,
            temp_workspace::TempWorkspace,
            utils::{sevenzip::SevenZip, utils::get_file_list},
        };
        let workspace = TempWorkspace::create("import")?;
        let extracted = workspace.path().join("source");
        let base = if source.is_file()
            && !source
                .extension()
                .is_some_and(|s| s.eq_ignore_ascii_case("inf"))
        {
            let zip = SevenZip::new()?;
            zip.is_driver_package(source, password)?;
            zip.extract_files_from_path(source, password, "*", &extracted)?;
            extracted.as_path()
        } else if source.is_file() {
            source.parent().unwrap_or(Path::new("."))
        } else {
            source
        };
        let mut paths = if source.is_file()
            && source
                .extension()
                .is_some_and(|s| s.eq_ignore_ascii_case("inf"))
        {
            vec![source.to_path_buf()]
        } else {
            get_file_list(base, "*.inf")?
        };
        paths.sort_by_key(|p| p.to_string_lossy().to_ascii_lowercase());
        if matched {
            let hardware = crate::hardware::enumerate_hardware(None, false)?;
            let infos: Vec<_> = paths
                .iter()
                .filter_map(|p| crate::driver_index::InfInfo::parse_inf(base, p).ok())
                .collect();
            let selected = crate::command::match_driver_info(&hardware, &infos, None, None);
            let mut keep = std::collections::BTreeSet::new();
            for (_, candidates) in selected {
                if let Some((info, _)) = candidates.first() {
                    keep.insert(base.join(&info.path));
                }
            }
            paths.retain(|p| keep.contains(p));
        }
        if paths.is_empty() {
            return Err(anyhow::anyhow!(rust_i18n::t!("no-found-driver-currently")));
        }
        let api = self.target.api()?;
        let backend = NativeBackend::new(&self.target, &api)?;
        Ok(import_infs(&backend, &paths, !self.target.offline))
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

pub fn report_batch(batch: &crate::driver_management::BatchResult) {
    use crate::{
        driver_management::ItemStatus,
        utils::console::{ConsoleType, write_console},
    };
    for item in &batch.items {
        let (kind, key) = match item.status {
            ItemStatus::Success => (ConsoleType::Success, "management.success"),
            ItemStatus::NotApplicable => (ConsoleType::Info, "management.not-applicable"),
            ItemStatus::Failed => (ConsoleType::Error, "management.failed"),
        };
        write_console(
            kind,
            &format!(
                "{}: {} {}{}",
                rust_i18n::t!(key),
                item.path.display(),
                item.message,
                item.code
                    .map(|n| format!(" [0x{n:08X}]"))
                    .unwrap_or_default()
            ),
        );
    }
    let (success, fail, not_applicable, total) = batch.counts();
    write_console(
        ConsoleType::Info,
        &rust_i18n::t!(
            "management.summary",
            success = success,
            fail = fail,
            not_applicable = not_applicable,
            total = total
        ),
    );
}
