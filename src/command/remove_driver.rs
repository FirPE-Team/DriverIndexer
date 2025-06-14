use crate::i18n::getLocaleText;
use crate::utils::drvstoreAPI::DriverStore;
use crate::utils::util::isOfflineSystem;
use std::error::Error;
use std::path::{Path, PathBuf};

pub fn remove_driver(systemDrive: &Path, driveName: &str) -> Result<(), Box<dyn Error>> {
    let systemRoot = systemDrive.join("Windows");

    unsafe {
        let driverStore = DriverStore::new()?;

        if isOfflineSystem(Path::new(systemDrive))? {
            // 离线删除驱动
            if let Some(infPath) = findInfFullPath(systemDrive, driveName) {
                driverStore.offline_delete_driver(&infPath, &systemRoot, systemDrive, 0)?;
            }
        } else {
            // 在线删除驱动

            // 查找inf完整路径
            let infPath = match findInfFullPath(systemDrive, driveName) {
                Some(p) => p,
                None => return Err(getLocaleText("no-inf-find", None).into())
            };
            let handle = driverStore.open_store(&systemRoot, systemDrive)?;
            let result = driverStore.delete_driver(handle, &infPath, 0);
            driverStore.close_store(handle).ok();
            result?
        }
    }
    Ok(())
}

/// 查找离线系统中指定inf文件名对应的完整路径
fn findInfFullPath(systemPath: &Path, inf_file_name: &str) -> Option<PathBuf> {
    let repo_dir = systemPath.join("Windows").join("System32").join("DriverStore").join("FileRepository");

    if !repo_dir.is_dir() {
        return None;
    }

    for entry in std::fs::read_dir(repo_dir).ok()? {
        let entry = entry.ok()?;
        let file_name = entry.file_name();
        let dir_name = file_name.to_string_lossy();

        if dir_name.starts_with(&inf_file_name.to_ascii_lowercase()) {
            let full_path = entry.path().join(inf_file_name);
            if full_path.exists() {
                return Some(full_path);
            }
        }
    }

    None
}
