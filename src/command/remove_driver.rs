use crate::i18n::getLocaleText;
use crate::utils::drvstoreAPI::DriverStore;
use crate::utils::util::{getArchCode, getFileList, isOfflineSystem};
use std::error::Error;
use std::path::{Path, PathBuf};

pub fn remove_driver(systemDrive: &Path, driveName: &str) -> Result<(), Box<dyn Error>> {
    let systemRoot = systemDrive.join("Windows");

    // 获取系统架构
    let archCode = match getArchCode(systemDrive) {
        Ok(code @ (0x014c | 0x8664 | 0xAA64)) => code,
        Err(_) | _ => {
            return Err(getLocaleText("offline-Arch-Err", None).into());
        }
    };
    let arch = match archCode {
        // x86
        0x014c => 0,
        // x64
        0x8664 => 9,
        // ARM64
        0xAA64 => 12,
        _ => { unreachable!() }
    };

    unsafe {
        let driverStore = DriverStore::new()?;

        let mut result = Err(getLocaleText("no-inf-find", None).into());
        let handle = driverStore.open_store(&systemRoot, systemDrive)?;

        for item in getFileList(&*systemRoot.join("INF"), "oem*.inf")? {
            if let Some((path, _info_opt)) = driverStore.find_driver_package(handle, &item, arch) {
                let inf_path = PathBuf::from(&path);
                if inf_path.file_name().unwrap().to_str().unwrap().to_lowercase() == driveName {
                    if isOfflineSystem(Path::new(systemDrive))? {
                        // 离线删除驱动
                        result = Ok(driverStore.offline_delete_driver(&inf_path, &systemRoot, systemDrive, 0)?)
                    } else {
                        // 在线删除驱动
                        result = Ok(driverStore.delete_driver(handle, &inf_path, 0)?);
                    }
                    break;
                }
            }
        }
        result
    }
}
