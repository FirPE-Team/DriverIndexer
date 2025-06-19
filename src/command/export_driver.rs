use crate::i18n::getLocaleText;
use crate::utils::drvstoreAPI::DriverStore;
use crate::utils::setupAPI;
use crate::utils::util::{copy_dir, getArchCode, getFileList};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub fn export_driver(systemDrive: &Path, outPath: &Path, name: Option<&str>, class: Option<&str>) -> Result<(), Box<dyn Error>> {
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

        // 遍历驱动库
        for item in getFileList(&*systemRoot.join("INF"), "oem*.inf")? {
            if let Some((path, info_opt)) = driverStore.find_driver_package(handle, &item, arch) {
                let inf_path = PathBuf::from(&path);

                // 获取驱动基本信息
                let driver_handle = driverStore.open_driver(&inf_path, arch)?;
                let driver_info = driverStore.get_version_info(driver_handle)?;

                // 指定驱动名称
                if let Some(name) = name {
                    if inf_path.file_name().unwrap().to_str().unwrap().to_lowercase() != name {
                        continue;
                    }
                }

                // 指定驱动类
                if let Some(class) = class {
                    if driver_info.class_name.to_lowercase() != class.to_lowercase() {
                        continue;
                    }
                }

                let result_path = outPath.join(setupAPI::get_class_description(driver_info.class_guid)?).join(inf_path.parent().unwrap().file_name().unwrap());
                fs::create_dir_all(&result_path)?;

                result = Ok(copy_dir(inf_path.parent().unwrap(), &result_path)?);
            }
        }
        result
    }
}
