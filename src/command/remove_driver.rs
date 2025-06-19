use crate::i18n::getLocaleText;
use crate::utils::console::{writeConsole, ConsoleType};
use crate::utils::drvstoreAPI::DriverStore;
use crate::utils::util::{getArchCode, getFileList, isOfflineSystem};
use fluent_templates::fluent_bundle::FluentValue;
use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};

pub fn remove_driver(systemDrive: &Path, driveName: Option<&str>, class: Option<&str>) -> Result<(), Box<dyn Error>> {
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

                // 获取驱动基本信息
                let driver_handle = driverStore.open_driver(&inf_path, arch)?;
                let driver_info = driverStore.get_version_info(driver_handle)?;

                // 指定驱动名称
                if let Some(name) = driveName {
                    if inf_path.file_name().unwrap().to_str().unwrap().to_lowercase() != name.to_lowercase() {
                        continue;
                    }
                }

                // 指定驱动类
                if let Some(class) = class {
                    if driver_info.class_name.to_lowercase() != class.to_lowercase() {
                        continue;
                    }
                }

                let args: HashMap<String, FluentValue> = hash_map!("inf".to_string() => inf_path.file_name().unwrap().to_str().unwrap().into());
                writeConsole(ConsoleType::Info, &getLocaleText("driver-remove", Some(&args)));

                if isOfflineSystem(Path::new(systemDrive))? {
                    // 离线删除驱动
                    result = Ok(driverStore.offline_delete_driver(&inf_path, &systemRoot, systemDrive, 0)?)
                } else {
                    // 在线删除驱动
                    result = Ok(driverStore.delete_driver(handle, &inf_path, 0)?);
                }
            }
        }
        result
    }
}
